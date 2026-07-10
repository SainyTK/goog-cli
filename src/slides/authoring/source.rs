use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeckSource {
    pub schema_version: u32,
    pub presentation: Value,
    pub theme: Value,
    #[serde(default)]
    pub assets: BTreeMap<String, Value>,
    #[serde(default)]
    pub layouts: BTreeMap<String, Value>,
    pub quality: Value,
    pub slides: Vec<SlideDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlideDefinition {
    pub key: String,
    pub pattern: String,
    #[serde(flatten)]
    pub content: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemaVersionProbe {
    schema_version: u32,
}

#[derive(Debug, Clone, Copy)]
enum DeckSourceFormat {
    Json,
    Yaml,
}

#[derive(Debug)]
pub struct DeckSourceError {
    message: String,
}

impl DeckSourceError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DeckSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for DeckSourceError {}

pub fn read_deck_source(
    path_or_stdin: &str,
    stdin: &mut impl Read,
) -> Result<DeckSource, DeckSourceError> {
    let (contents, source_name) = if path_or_stdin == "-" {
        let mut contents = String::new();
        stdin.read_to_string(&mut contents).map_err(|error| {
            DeckSourceError::new(format!("failed to read Deck Source from stdin: {error}"))
        })?;
        (contents, "stdin".to_string())
    } else {
        let contents = std::fs::read_to_string(path_or_stdin).map_err(|error| {
            DeckSourceError::new(format!(
                "failed to read Deck Source {path_or_stdin}: {error}"
            ))
        })?;
        (contents, path_or_stdin.to_string())
    };

    let format = if path_or_stdin == "-" {
        if serde_json::from_str::<SchemaVersionProbe>(&contents).is_ok() {
            DeckSourceFormat::Json
        } else {
            DeckSourceFormat::Yaml
        }
    } else {
        match Path::new(path_or_stdin)
            .extension()
            .and_then(|extension| extension.to_str())
        {
            Some("json") => DeckSourceFormat::Json,
            Some("yaml" | "yml") => DeckSourceFormat::Yaml,
            _ => {
                return Err(DeckSourceError::new(format!(
                    "Deck Source path must end in .json, .yaml, or .yml: {source_name}"
                )))
            }
        }
    };

    let schema_version = match format {
        DeckSourceFormat::Json => serde_json::from_str::<SchemaVersionProbe>(&contents)
            .map(|probe| probe.schema_version)
            .map_err(|error| {
                DeckSourceError::new(format!(
                    "failed to parse JSON Deck Source {source_name}: {error}"
                ))
            })?,
        DeckSourceFormat::Yaml => serde_yaml_ng::from_str::<SchemaVersionProbe>(&contents)
            .map(|probe| probe.schema_version)
            .map_err(|error| {
                DeckSourceError::new(format!(
                    "failed to parse YAML Deck Source {source_name}: {error}"
                ))
            })?,
    };

    if schema_version != 1 {
        return Err(DeckSourceError::new(format!(
            "unsupported Deck Source schemaVersion {schema_version} in {source_name}; supported version: 1"
        )));
    }

    let source: DeckSource = match format {
        DeckSourceFormat::Json => serde_json::from_str(&contents).map_err(|error| {
            DeckSourceError::new(format!(
                "failed to parse JSON Deck Source {source_name}: {error}"
            ))
        })?,
        DeckSourceFormat::Yaml => serde_yaml_ng::from_str(&contents).map_err(|error| {
            DeckSourceError::new(format!(
                "failed to parse YAML Deck Source {source_name}: {error}"
            ))
        })?,
    };

    let mut slide_keys = BTreeMap::new();
    for (index, slide) in source.slides.iter().enumerate() {
        if let Some(first_index) = slide_keys.insert(slide.key.as_str(), index) {
            return Err(DeckSourceError::new(format!(
                "duplicate slide key '{}' at slides[{index}].key in {source_name}; first declared at slides[{first_index}].key",
                slide.key
            )));
        }
    }

    Ok(source)
}
