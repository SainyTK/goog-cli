use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;
use std::path::Path;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeckSource {
    pub schema_version: u32,
    pub presentation: PresentationDefinition,
    pub theme: ThemeDefinition,
    #[serde(default)]
    pub assets: BTreeMap<String, AssetDefinition>,
    #[serde(default)]
    pub layouts: BTreeMap<String, Value>,
    pub quality: QualityDefinition,
    pub slides: Vec<SlideDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub url: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub checksum: String,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub alt_text: Option<String>,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub placement_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeDefinition {
    #[serde(default, deserialize_with = "deserialize_strict_string_map")]
    pub colors: BTreeMap<String, String>,
    #[serde(default)]
    pub fonts: BTreeMap<String, FontDefinition>,
    #[serde(default)]
    pub type_styles: BTreeMap<String, TypeStyleDefinition>,
    #[serde(default, deserialize_with = "deserialize_finite_number_map")]
    pub spacing: BTreeMap<String, f64>,
    #[serde(default, deserialize_with = "deserialize_strict_string_map")]
    pub fills: BTreeMap<String, String>,
    #[serde(default)]
    pub outlines: BTreeMap<String, OutlineDefinition>,
    #[serde(default)]
    pub lines: BTreeMap<String, LineDefinition>,
    pub geometry: Option<GeometryDefinition>,
    pub pattern_defaults: Option<PatternDefaultsDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontDefinition {
    pub family: String,
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TypeStyleDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub font: String,
    pub size: f64,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub weight: Option<String>,
    pub line_spacing: f64,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub alignment: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutlineDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub color: String,
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub width: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub color: String,
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub width: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeometryDefinition {
    pub safe_area: Option<GeometrySafeAreaDefinition>,
    pub footer: Option<FooterGeometryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeometrySafeAreaDefinition {
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub top: f64,
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub right: f64,
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub bottom: f64,
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub left: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FooterGeometryDefinition {
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub height: f64,
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub gap: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatternDefaultsDefinition {
    pub footer: Option<FooterDefaultsDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FooterDefaultsDefinition {
    pub show_slide_number: bool,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub line: String,
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

fn deserialize_optional_strict_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<StrictString>::deserialize(deserializer).map(|value| value.map(|value| value.0))
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

fn deserialize_finite_number_map<'de, D>(deserializer: D) -> Result<BTreeMap<String, f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = BTreeMap::<String, FiniteNumber>::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .map(|(key, value)| (key, value.0))
        .collect())
}

fn deserialize_finite_number<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    FiniteNumber::deserialize(deserializer).map(|value| value.0)
}

fn deserialize_optional_finite_number<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<FiniteNumber>::deserialize(deserializer).map(|value| value.map(|value| value.0))
}

struct FiniteNumber(f64);

impl<'de> Deserialize<'de> for FiniteNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(de::Error::custom("number must be finite"))
        }
    }
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
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub key: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub pattern: String,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub eyebrow: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub subtitle: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub footer: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub statement: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub body: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub takeaway: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub owner: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub emphasis: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub density: Option<String>,
    #[serde(default)]
    pub items: Vec<ListItemDefinition>,
    pub columns: Option<SlideColumnsDefinition>,
    #[serde(default)]
    pub rows: Vec<EvidenceTableRowDefinition>,
    #[serde(default)]
    pub stages: Vec<ProcessStageDefinition>,
    pub evidence: Option<SlideEvidenceDefinition>,
    #[serde(default)]
    pub groups: Vec<CardGroupDefinition>,
    #[serde(default)]
    pub steps: Vec<StepDefinition>,
    #[serde(default)]
    pub signals: Vec<TimelineSignalDefinition>,
    #[serde(default)]
    pub milestones: Vec<TimelineMilestoneDefinition>,
    #[serde(default)]
    pub sources: Vec<SourceDefinition>,
    #[serde(default)]
    pub questions: Vec<QuestionDefinition>,
    #[serde(flatten)]
    pub content: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListItemDefinition {
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub key: Option<String>,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub title: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlideEvidenceDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub title: String,
    #[serde(default)]
    pub items: Vec<ListItemDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardGroupDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub key: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub title: String,
    #[serde(default)]
    pub cards: Vec<CardDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub key: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub title: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StepDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub key: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub title: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineSignalDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub key: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_strict_string_vec")]
    pub items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineMilestoneDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub key: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub body: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub exit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub key: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub title: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub key: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub title: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProcessStageDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub key: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub body: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub test: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub measure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvidenceTableRowDefinition {
    pub key: String,
    #[serde(flatten)]
    pub cells: BTreeMap<String, String>,
}

impl<'de> Deserialize<'de> for EvidenceTableRowDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(EvidenceTableRowDefinitionVisitor)
    }
}

struct EvidenceTableRowDefinitionVisitor;

impl<'de> Visitor<'de> for EvidenceTableRowDefinitionVisitor {
    type Value = EvidenceTableRowDefinition;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an evidence table row with a stable key and string cells")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut key = None;
        let mut cells = BTreeMap::new();

        while let Some(field) = map.next_key::<String>()? {
            if field == "key" {
                if key.is_some() {
                    return Err(de::Error::duplicate_field("key"));
                }
                key = Some(map.next_value::<StrictString>()?.0);
            } else {
                let value = map.next_value::<StrictString>()?.0;
                cells.insert(field, value);
            }
        }

        let key = key.ok_or_else(|| de::Error::missing_field("key"))?;
        Ok(EvidenceTableRowDefinition { key, cells })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SlideColumnsDefinition {
    Count(u32),
    Definitions(Vec<SlideColumnDefinition>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlideColumnDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub key: String,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub summary: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub body: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_strict_string")]
    pub label: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_finite_number")]
    pub width: Option<f64>,
    #[serde(default)]
    pub sections: Vec<SlideColumnSectionDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlideColumnSectionDefinition {
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub label: String,
    #[serde(deserialize_with = "deserialize_strict_string")]
    pub body: String,
}

impl<'de> Deserialize<'de> for SlideColumnsDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SlideColumnsDefinitionVisitor)
    }
}

struct SlideColumnsDefinitionVisitor;

impl<'de> Visitor<'de> for SlideColumnsDefinitionVisitor {
    type Value = SlideColumnsDefinition;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a non-negative integer column count or a list of column definitions")
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let count = u32::try_from(value)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Unsigned(value), &self))?;
        Ok(SlideColumnsDefinition::Count(count))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let count = u32::try_from(value)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Signed(value), &self))?;
        Ok(SlideColumnsDefinition::Count(count))
    }

    fn visit_seq<S>(self, mut sequence: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut definitions = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
        while let Some(definition) = sequence.next_element()? {
            definitions.push(definition);
        }
        Ok(SlideColumnsDefinition::Definitions(definitions))
    }
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
