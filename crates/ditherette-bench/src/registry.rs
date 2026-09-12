//! Subject registry lookup and path resolution.

use ditherette_bench_api::{BenchSubject, ResizeBenchSubject, SubjectId};

use crate::{
    cli::{csv_set, Flags},
    error::BenchError,
    util::normalize_path_string,
};

#[derive(Clone)]
pub(crate) struct Registry {
    subjects: Vec<BenchSubject>,
}

impl Registry {
    pub(crate) fn load() -> Self {
        Self {
            subjects: ditherette_wasm::bench_subjects(),
        }
    }

    pub(crate) fn subjects(&self) -> &[BenchSubject] {
        &self.subjects
    }

    pub(crate) fn resize_subject(&self, id: &str) -> Result<ResizeBenchSubject, BenchError> {
        self.subjects
            .iter()
            .find_map(|subject| match subject {
                BenchSubject::Resize(resize) if resize.descriptor.id.as_str() == id => {
                    Some(resize.clone())
                }
                _ => None,
            })
            .ok_or_else(|| BenchError::Config(format!("unknown resize subject {id:?}")))
    }

    pub(crate) fn select_resize_subjects(
        &self,
        flags: &Flags,
    ) -> Result<Vec<ResizeBenchSubject>, BenchError> {
        if let Some(subjects) = flags.optional("--subjects") {
            return subjects
                .split(',')
                .filter(|value| !value.is_empty())
                .map(|id| self.resize_subject(id))
                .collect();
        }

        let module = flags.optional("--module");
        let filter_set = flags.optional("--filters").map(csv_set);
        let variant_set = flags.optional("--variants").map(csv_set);

        Ok(self
            .subjects
            .iter()
            .map(|subject| match subject {
                BenchSubject::Resize(resize) => resize,
            })
            .filter(|subject| {
                let id = &subject.descriptor.id;
                module.is_none_or(|module| id.module() == module)
                    && filter_set
                        .as_ref()
                        .is_none_or(|filters| filters.iter().any(|filter| filter == id.filter()))
                    && variant_set.as_ref().is_none_or(|variants| {
                        variants.iter().any(|variant| variant == id.variant())
                    })
            })
            .cloned()
            .collect())
    }

    pub(crate) fn resolve_resize_subject_or_path(
        &self,
        value: &str,
    ) -> Result<ResizeBenchSubject, BenchError> {
        if SubjectId::parse(value).is_ok() {
            return self.resize_subject(value);
        }

        let path = normalize_path_string(value);
        let matches = self
            .subjects
            .iter()
            .filter_map(|subject| match subject {
                BenchSubject::Resize(resize)
                    if normalize_path_string(&resize.descriptor.source_file) == path =>
                {
                    Some(resize.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        match matches.len() {
            0 => Err(BenchError::Config(format!(
                "no registered resize subject for path {value:?}"
            ))),
            1 => Ok(matches[0].clone()),
            _ => Err(BenchError::Config(format!(
                "path {value:?} maps to multiple subjects: {}",
                matches
                    .iter()
                    .map(|subject| subject.descriptor.id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }
}
