use std::fs::metadata;
use std::io::Error as IOError;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::Missing;

pub enum OutputStatus {
    UpToDate,
    FileMissing,
    OutOfDate,
    Forced,
    CannotDetermine(IOError),
}

use OutputStatus::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TemplateSpec {
    pub name: String,
    pub data: PathBuf,
    pub template: PathBuf,
    pub output: Option<PathBuf>,
}

fn get_mod_time(p: impl AsRef<Path>) -> Result<SystemTime, IOError> {
    metadata(p)?.modified()
}

impl TemplateSpec {
    pub fn new<S, P>(name: S, data: P, template: P, output: Option<P>) -> Result<Self, Missing>
    where
        P: Into<PathBuf>,
        S: Into<String>,
    {
        let spec = Self::new_unchecked(
            name.into(),
            data.into(),
            template.into(),
            output.map(|x| x.into()),
        );

        spec.validate_files()?;
        Ok(spec)
    }

    pub fn new_unchecked(
        name: String,
        data: PathBuf,
        template: PathBuf,
        output: Option<PathBuf>,
    ) -> Self {
        Self {
            name,
            data,
            template,
            output,
        }
    }

    pub fn validate_files(&self) -> Result<(), Missing> {
        let data_exists = self.data.exists();
        let template_exists = self.template.exists();

        match (data_exists, template_exists) {
            (true, true) => Ok(()),
            (data, template) => {
                let mut missing = Vec::new();
                if template {
                    missing.push(format!("template file: {}", self.template.display()));
                }
                if data {
                    missing.push(format!("data file: {}", self.data.display()));
                }
                Err(missing.into())
            }
        }
    }

    pub fn should_build(&self) -> bool {
        if let UpToDate = self.up_to_date() {
            false
        } else {
            true
        }
    }

    pub fn up_to_date(&self) -> OutputStatus {
        let output = match &self.output {
            Some(o) => o,
            None => {
                return Forced;
            }
        };

        if !output.exists() {
            return FileMissing;
        }

        let output_modified = match get_mod_time(&output) {
            Ok(t) => t,
            Err(e) => {
                return CannotDetermine(e);
            }
        };

        let data_modified = match get_mod_time(&self.data) {
            Ok(t) => t,
            Err(e) => {
                return CannotDetermine(e);
            }
        };

        let template_modified = match get_mod_time(&self.template) {
            Ok(t) => t,
            Err(e) => {
                return CannotDetermine(e);
            }
        };

        if output_modified < template_modified || output_modified < data_modified {
            OutOfDate
        } else {
            UpToDate
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json;

    #[test]
    fn deser_single() {
        let actual: TemplateSpec = serde_json::from_value(serde_json::json!({
            "name": "example",
            "data": "example.json",
            "template": "example.hbs",
            "output": "example.rst"
        }))
        .unwrap();

        let expected = TemplateSpec::new_unchecked(
            "example".into(),
            "example.json".into(),
            "example.hbs".into(),
            Some("example.rst".into()),
        );

        assert_eq!(actual, expected);
    }
}