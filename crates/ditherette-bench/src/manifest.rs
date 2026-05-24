//! Declarative benchmark manifest loading and CLI argument expansion.

use std::{fs, path::PathBuf};

use toml::Value;

use crate::error::BenchError;

const DEFAULT_MANIFEST: &str = "ditherette-bench.toml";

pub(crate) struct ExpandedCommand {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
}

pub(crate) fn expand_command(
    command: String,
    args: Vec<String>,
) -> Result<ExpandedCommand, BenchError> {
    if command == "run" {
        return expand_run_command(args);
    }

    let (profile, config_path, args_without_manifest_flags) = extract_manifest_flags(args)?;
    if profile.is_none() && !matches!(command.as_str(), "perf" | "comp" | "tile" | "tiling-sweep") {
        return Ok(ExpandedCommand {
            command,
            args: args_without_manifest_flags,
        });
    }
    let Some(manifest) = BenchManifest::load_optional(config_path)? else {
        return Ok(ExpandedCommand {
            command,
            args: args_without_manifest_flags,
        });
    };

    let mut expanded = Vec::new();
    let skip_positionals = if command_requires_domain(&command) {
        let domain = args_without_manifest_flags.first().cloned();
        if let Some(domain) = domain {
            expanded.push(domain);
        }
        1
    } else {
        0
    };
    expanded.extend(manifest.defaults_args()?);
    if let Some(profile) = profile {
        expanded.extend(manifest.profile_args(&profile, false)?.args);
    }
    expanded.extend(
        args_without_manifest_flags
            .into_iter()
            .skip(skip_positionals),
    );

    Ok(ExpandedCommand {
        command,
        args: expanded,
    })
}

fn expand_run_command(args: Vec<String>) -> Result<ExpandedCommand, BenchError> {
    let profile = args
        .first()
        .ok_or_else(|| BenchError::Config("run requires a profile name".to_owned()))?
        .to_owned();
    let rest = args[1..].to_vec();
    let (_, config_path, args_without_manifest_flags) = extract_manifest_flags(rest)?;
    let manifest = BenchManifest::load_optional(config_path)?.ok_or_else(|| {
        BenchError::Config(format!(
            "no benchmark manifest found at {}; create one or pass --config PATH",
            default_manifest_path().display()
        ))
    })?;

    let profile = manifest.profile_args(&profile, true)?;
    let command = profile
        .command
        .ok_or_else(|| BenchError::Config("run profile requires command".to_owned()))?;
    let mut expanded_args = Vec::new();
    if command_requires_domain(&command) {
        expanded_args.push(profile.domain.ok_or_else(|| {
            BenchError::Config(format!(
                "run profile for {command:?} requires domain = \"resize\""
            ))
        })?);
    }
    expanded_args.extend(manifest.defaults_args()?);
    expanded_args.extend(profile.args);
    expanded_args.extend(args_without_manifest_flags);

    Ok(ExpandedCommand {
        command,
        args: expanded_args,
    })
}

fn command_requires_domain(command: &str) -> bool {
    matches!(command, "perf" | "comp" | "tile")
}

struct BenchManifest {
    root: Value,
}

impl BenchManifest {
    fn load_optional(config_path: Option<PathBuf>) -> Result<Option<Self>, BenchError> {
        let path = config_path.unwrap_or_else(default_manifest_path);
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(BenchError::io)?;
        let root = toml::from_str::<Value>(&text).map_err(|error| {
            BenchError::Config(format!("failed to parse {}: {error}", path.display()))
        })?;
        Ok(Some(Self { root }))
    }

    fn defaults_args(&self) -> Result<Vec<String>, BenchError> {
        let Some(defaults) = self.root.get("defaults") else {
            return Ok(Vec::new());
        };
        table_to_args(defaults, false)
    }

    fn profile_args(
        &self,
        name: &str,
        include_positionals: bool,
    ) -> Result<ProfileArgs, BenchError> {
        let profile = self
            .root
            .get("profiles")
            .and_then(|profiles| profiles.get(name))
            .ok_or_else(|| BenchError::Config(format!("unknown benchmark profile {name:?}")))?;
        let command = profile
            .get("command")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let domain = profile
            .get("domain")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let args = table_to_args(profile, include_positionals)?;
        Ok(ProfileArgs {
            command,
            domain,
            args,
        })
    }
}

struct ProfileArgs {
    command: Option<String>,
    domain: Option<String>,
    args: Vec<String>,
}

fn table_to_args(value: &Value, skip_positionals: bool) -> Result<Vec<String>, BenchError> {
    let table = value
        .as_table()
        .ok_or_else(|| BenchError::Config("manifest sections must be TOML tables".to_owned()))?;
    let mut args = Vec::new();
    for (key, value) in table {
        if matches!(key.as_str(), "command" | "domain") {
            continue;
        }
        if skip_positionals && matches!(key.as_str(), "profile" | "config") {
            continue;
        }
        push_flag(&mut args, key, value)?;
    }
    Ok(args)
}

fn push_flag(args: &mut Vec<String>, key: &str, value: &Value) -> Result<(), BenchError> {
    let flag = format!("--{}", key.replace('_', "-"));
    match value {
        Value::Boolean(value) => {
            args.push(flag);
            args.push(value.to_string());
        }
        Value::String(value) => {
            args.push(flag);
            args.push(value.clone());
        }
        Value::Integer(value) => {
            args.push(flag);
            args.push(value.to_string());
        }
        Value::Float(value) => {
            args.push(flag);
            args.push(value.to_string());
        }
        Value::Array(values) => {
            args.push(flag);
            args.push(
                values
                    .iter()
                    .map(value_to_cli_string)
                    .collect::<Result<Vec<_>, _>>()?
                    .join(","),
            );
        }
        _ => {
            return Err(BenchError::Config(format!(
                "manifest key {key:?} must be a string, number, boolean, or array"
            )));
        }
    }
    Ok(())
}

fn value_to_cli_string(value: &Value) -> Result<String, BenchError> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Integer(value) => Ok(value.to_string()),
        Value::Float(value) => Ok(value.to_string()),
        Value::Boolean(value) => Ok(value.to_string()),
        _ => Err(BenchError::Config(
            "manifest arrays may contain only strings, numbers, or booleans".to_owned(),
        )),
    }
}

fn extract_manifest_flags(
    args: Vec<String>,
) -> Result<(Option<String>, Option<PathBuf>, Vec<String>), BenchError> {
    let mut profile = None;
    let mut config_path = None;
    let mut rest = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--profile" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    BenchError::Config("--profile requires a profile name".to_owned())
                })?;
                profile = Some(value.clone());
                index += 2;
            }
            value if value.starts_with("--profile=") => {
                profile = Some(value.trim_start_matches("--profile=").to_owned());
                index += 1;
            }
            "--config" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    BenchError::Config("--config requires a manifest path".to_owned())
                })?;
                config_path = Some(PathBuf::from(value));
                index += 2;
            }
            value if value.starts_with("--config=") => {
                config_path = Some(PathBuf::from(value.trim_start_matches("--config=")));
                index += 1;
            }
            _ => {
                rest.push(args[index].clone());
                index += 1;
            }
        }
    }
    Ok((profile, config_path, rest))
}

fn default_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_MANIFEST)
}
