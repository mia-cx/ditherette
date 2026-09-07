//! Syntax restrictions close source-injection escapes. Rust itself checks dependency isolation.
use std::{env, fs, path::Path};

use quote::ToTokens;
use syn::{visit::Visit, Attribute, Item, UseTree};

struct Check<'a> {
    forbidden: &'a [&'a str],
    errors: Vec<String>,
}

impl<'ast> Visit<'ast> for Check<'_> {
    fn visit_ident(&mut self, ident: &'ast syn::Ident) {
        if self
            .forbidden
            .contains(&ident.to_string().trim_start_matches("r#"))
        {
            self.errors
                .push(format!("forbidden semantic identifier: {ident}"));
        }
    }

    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        let name = attr.path().to_token_stream().to_string();
        // Other attributes/derives remain available. Only source redirection and
        // macros escaping a semantic module need this additional restriction.
        if ["path", "cfg_attr", "macro_use", "macro_export"].contains(&name.as_str()) {
            self.errors
                .push(format!("source-injection attribute: {name}"));
        }
        syn::visit::visit_attribute(self, attr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let name = mac.path.to_token_stream().to_string();
        let injection = [
            "include",
            "include_str",
            "include_bytes",
            "env",
            "option_env",
            "path",
            "cfg_attr",
            "macro_use",
            "macro_export",
        ];
        if mac
            .path
            .segments
            .iter()
            .any(|s| injection.contains(&s.ident.to_string().as_str()))
        {
            self.errors.push(format!("source-injection macro: {name}"));
        }
        // syn does not parse macro arguments. Tokenize through nested groups too.
        fn inspect(tokens: proc_tokens::TokenStream, forbidden: &[&str], errors: &mut Vec<String>) {
            for token in tokens {
                match token {
                    proc_tokens::TokenTree::Group(group) => {
                        inspect(group.stream(), forbidden, errors)
                    }
                    proc_tokens::TokenTree::Ident(ident)
                        if forbidden.contains(&ident.to_string().trim_start_matches("r#")) =>
                    {
                        errors.push(format!("forbidden macro token: {ident}"))
                    }
                    _ => {}
                }
            }
        }
        // Built-in formatting/assertion macros cannot generate attributes. A
        // local macro definition or invocation can, so its tokens also reject path.
        let token_injection: Vec<_> = injection
            .iter()
            .copied()
            .filter(|token| {
                *token != "path"
                    || ![
                        "write",
                        "writeln",
                        "format",
                        "assert",
                        "assert_eq",
                        "debug_assert",
                        "debug_assert_eq",
                    ]
                    .contains(&name.as_str())
            })
            .collect();
        let forbidden = [self.forbidden, &token_injection].concat();
        inspect(mac.tokens.clone(), &forbidden, &mut self.errors);
    }
}

fn root(file: &syn::File) -> Result<(), String> {
    let mut modules = Vec::new();
    for item in &file.items {
        match item {
            Item::Mod(module) if module.content.is_none() => {
                let name = module.ident.to_string();
                let attrs = module
                    .attrs
                    .iter()
                    .map(|a| a.to_token_stream().to_string())
                    .collect::<Vec<_>>();
                let expected: Vec<String> = if name == "bench_subjects" {
                    vec!["# [cfg (feature = \"bench-subjects\")]".into()]
                } else {
                    vec![]
                };
                if attrs != expected {
                    return Err(format!("changed module wiring: {name}"));
                }
                modules.push(name);
            }
            Item::Use(import) => {
                let UseTree::Path(path) = &import.tree else {
                    return Err("root reexports must name their module".into());
                };
                if !["wasm", "bench_subjects", "wasm_bindgen_rayon"]
                    .contains(&path.ident.to_string().as_str())
                {
                    return Err("root semantic aliases/reexports are forbidden".into());
                }
                // Prevent aliasing one of these namespaces to image/spec/serde/etc.
                fn no_alias(tree: &UseTree) -> bool {
                    match tree {
                        UseTree::Rename(_) | UseTree::Glob(_) => false,
                        UseTree::Path(p) => no_alias(&p.tree),
                        UseTree::Group(g) => g.items.iter().all(no_alias),
                        _ => true,
                    }
                }
                if !no_alias(&import.tree) {
                    return Err("root aliases/glob reexports are forbidden".into());
                }
            }
            _ => {
                return Err(
                    "crate root allows only audited module declarations and adapter reexports"
                        .into(),
                )
            }
        }
    }
    modules.sort();
    if modules != ["bench_subjects", "image", "prod", "spec", "wasm"] {
        return Err("changed crate module set".into());
    }
    Ok(())
}

fn check(path: &Path, role: &str) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if role == "core-manifest" || role == "bench-manifest" {
        let manifest: toml::Table = toml::from_str(&source).map_err(|e| e.to_string())?;
        for key in ["patch", "replace", "workspace"] {
            if manifest.contains_key(key) {
                return Err(format!("unaudited Cargo {key}"));
            }
        }
        let expected: toml::Table = toml::from_str(if role == "core-manifest" {
            "[release]\nopt-level = 's'"
        } else {
            ""
        })
        .unwrap();
        let profiles = manifest
            .get("profile")
            .and_then(toml::Value::as_table)
            .cloned()
            .unwrap_or_default();
        if profiles != expected {
            return Err("reference build profiles changed (including overflow checks)".into());
        }
        if role == "core-manifest"
            && manifest
                .get("package")
                .and_then(|p| p.get("build"))
                .is_some_and(|v| v != &toml::Value::Boolean(false))
        {
            return Err("reference build script changed".into());
        }
        return Ok(());
    }
    let file = syn::parse_file(&source).map_err(|e| e.to_string())?;
    let forbidden: &[&str] = match role {
        "spec" => &["prod", "wasm", "bench_subjects"],
        "prod" => &["spec", "wasm", "bench_subjects"],
        "image" => &["spec", "prod", "wasm", "bench_subjects"],
        "root" => &[],
        _ => return Err("unknown source role".into()),
    };
    let mut check = Check {
        forbidden,
        errors: vec![],
    };
    check.visit_file(&file);
    if role == "root" {
        root(&file)?;
    }
    if check.errors.is_empty() {
        Ok(())
    } else {
        Err(check.errors.join("; "))
    }
}

fn main() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() % 2 != 0 || args.is_empty() {
        return Err("expected pairs: ROLE PATH".into());
    }
    for pair in args.chunks_exact(2) {
        check(Path::new(&pair[1]), &pair[0]).map_err(|error| format!("{}: {error}", pair[1]))?;
    }
    Ok(())
}
