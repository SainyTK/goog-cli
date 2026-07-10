use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;
use std::path::Path;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeckSource {
    pub schema_version: u32,
    pub presentation: PresentationDefinition,
    pub theme: ThemeDefinition,
    #[serde(default)]
    pub assets: BTreeMap<String, Value>,
    #[serde(default)]
    pub layouts: BTreeMap<String, Value>,
    pub quality: QualityDefinition,
    pub slides: Vec<SlideDefinition>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeDefinition {
    #[serde(default, deserialize_with = "deserialize_strict_string_map")]
    pub colors: BTreeMap<String, String>,
    #[serde(default)]
    pub fonts: BTreeMap<String, FontDefinition>,
    #[serde(default)]
    pub type_styles: BTreeMap<String, Value>,
    #[serde(default)]
    pub spacing: BTreeMap<String, Value>,
    #[serde(default)]
    pub fills: BTreeMap<String, Value>,
    #[serde(default)]
    pub outlines: BTreeMap<String, Value>,
    #[serde(default)]
    pub lines: BTreeMap<String, Value>,
    pub geometry: Option<Value>,
    pub pattern_defaults: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontDefinition {
    pub family: String,
    pub fallbacks: Vec<String>,
}

impl<'de> Deserialize<'de> for FontDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(FontDefinitionVisitor)
    }
}

struct FontDefinitionVisitor;

impl<'de> Visitor<'de> for FontDefinitionVisitor {
    type Value = FontDefinition;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a font family string or font definition")
    }

    fn visit_str<E>(self, family: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(FontDefinition {
            family: family.to_owned(),
            fallbacks: Vec::new(),
        })
    }

    fn visit_string<E>(self, family: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(FontDefinition {
            family,
            fallbacks: Vec::new(),
        })
    }

    fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let definition =
            FontDefinitionObject::deserialize(de::value::MapAccessDeserializer::new(map))?;
        Ok(FontDefinition {
            family: definition.family,
            fallbacks: definition.fallbacks,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FontDefinitionObject {
    #[serde(deserialize_with = "deserialize_strict_string")]
    family: String,
    #[serde(default, deserialize_with = "deserialize_strict_string_vec")]
    fallbacks: Vec<String>,
}

fn deserialize_strict_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    StrictString::deserialize(deserializer).map(|value| value.0)
}

fn deserialize_strict_string_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<StrictString>::deserialize(deserializer)?;
    Ok(values.into_iter().map(|value| value.0).collect())
}

fn deserialize_strict_string_map<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = BTreeMap::<String, StrictString>::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .map(|(key, value)| (key, value.0))
        .collect())
}

struct StrictString(String);

impl<'de> Deserialize<'de> for StrictString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictStringVisitor)
    }
}

struct StrictStringVisitor;

impl Visitor<'_> for StrictStringVisitor {
    type Value = StrictString;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(StrictString(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(StrictString(value))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentationDefinition {
    pub aspect_ratio: Option<String>,
    pub language: Option<String>,
    pub speaker_notes: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualityDefinition {
    pub minimum_font_size: Option<f64>,
    pub minimum_text_contrast: Option<f64>,
    pub safe_area: Option<SafeAreaDefinition>,
    pub required_alt_text: Option<bool>,
    #[serde(default)]
    pub allowed_overlap_groups: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SafeAreaDefinition {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
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
        DeckSourceFormat::Json => {
            let mut deserializer = serde_json::Deserializer::from_str(&contents);
            serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
                DeckSourceError::new(format!(
                    "failed to parse JSON Deck Source {source_name} at {}: {}",
                    error.path(),
                    error.inner()
                ))
            })?
        }
        DeckSourceFormat::Yaml => {
            let deserializer = serde_yaml_ng::Deserializer::from_str(&contents);
            serde_path_to_error::deserialize(deserializer).map_err(|error| {
                DeckSourceError::new(format!(
                    "failed to parse YAML Deck Source {source_name} at {}: {}",
                    error.path(),
                    error.inner()
                ))
            })?
        }
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
