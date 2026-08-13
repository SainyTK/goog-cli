use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageGeometry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    pub section_start_index: i64,
    pub page_width_points: f64,
    pub page_height_points: f64,
    pub margin_top_points: f64,
    pub margin_bottom_points: f64,
    pub margin_left_points: f64,
    pub margin_right_points: f64,
    pub available_width_points: f64,
    pub available_height_points: f64,
}

pub(crate) fn resolve_body_page_geometry(
    document: &Value,
    tab_id: Option<&str>,
    insertion_index: i64,
) -> Result<PageGeometry> {
    if insertion_index < 0 {
        bail!("Google Docs insertion index must not be negative");
    }

    let (document_tab, resolved_tab_id) = select_document_tab(document, tab_id)?;
    let document_style = document_tab
        .get("documentStyle")
        .context("Google Docs response omitted documentStyle")?;
    if document_mode(document_style) == Some("PAGELESS") {
        bail!(
            "--fit-page is unavailable for pageless documents; provide --max-width and --max-height"
        );
    }

    let page_size = document_style
        .get("pageSize")
        .context("Google Docs response omitted pageSize; the document may be pageless")?;
    let page_width_points = dimension_points(page_size, "width", "page width")?;
    let page_height_points = dimension_points(page_size, "height", "page height")?;

    let body_content = document_tab
        .get("body")
        .and_then(|body| body.get("content"))
        .and_then(Value::as_array)
        .context("Google Docs response omitted body content")?;
    let empty_section_style = Value::Object(serde_json::Map::new());
    let section = body_content
        .iter()
        .filter_map(|element| {
            let start_index = element.get("startIndex")?.as_i64()?;
            let style = element.get("sectionBreak")?.get("sectionStyle")?;
            (start_index <= insertion_index).then_some((start_index, style))
        })
        .max_by_key(|(start_index, _)| *start_index)
        .unwrap_or((0, &empty_section_style));

    let margin_top_points =
        inherited_dimension_points(section.1, document_style, "marginTop", "top margin")?;
    let margin_bottom_points =
        inherited_dimension_points(section.1, document_style, "marginBottom", "bottom margin")?;
    let margin_left_points =
        inherited_dimension_points(section.1, document_style, "marginLeft", "left margin")?;
    let margin_right_points =
        inherited_dimension_points(section.1, document_style, "marginRight", "right margin")?;
    let available_width_points =
        round_points(page_width_points - margin_left_points - margin_right_points);
    let available_height_points =
        round_points(page_height_points - margin_top_points - margin_bottom_points);
    if available_width_points <= 0.0 || available_height_points <= 0.0 {
        bail!("page margins leave no positive body area for --fit-page");
    }

    Ok(PageGeometry {
        tab_id: resolved_tab_id,
        section_start_index: section.0,
        page_width_points,
        page_height_points,
        margin_top_points,
        margin_bottom_points,
        margin_left_points,
        margin_right_points,
        available_width_points,
        available_height_points,
    })
}

fn select_document_tab<'a>(
    document: &'a Value,
    requested_tab_id: Option<&str>,
) -> Result<(&'a Value, Option<String>)> {
    let tabs = document.get("tabs").and_then(Value::as_array);
    if let Some(tabs) = tabs.filter(|tabs| !tabs.is_empty()) {
        let selected = if let Some(requested_tab_id) = requested_tab_id {
            find_tab(tabs, requested_tab_id)
                .with_context(|| format!("Google Docs tab {requested_tab_id:?} was not found"))?
        } else {
            &tabs[0]
        };
        let resolved_tab_id = selected
            .get("tabProperties")
            .and_then(|properties| properties.get("tabId"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let document_tab = selected
            .get("documentTab")
            .context("selected Google Docs tab omitted documentTab")?;
        return Ok((document_tab, resolved_tab_id));
    }

    if requested_tab_id.is_some() {
        bail!("Google Docs response did not include tabs");
    }
    Ok((document, None))
}

fn find_tab<'a>(tabs: &'a [Value], tab_id: &str) -> Option<&'a Value> {
    tabs.iter().find_map(|tab| {
        let matches = tab
            .get("tabProperties")
            .and_then(|properties| properties.get("tabId"))
            .and_then(Value::as_str)
            == Some(tab_id);
        matches.then_some(tab).or_else(|| {
            tab.get("childTabs")
                .and_then(Value::as_array)
                .and_then(|children| find_tab(children, tab_id))
        })
    })
}

fn document_mode(document_style: &Value) -> Option<&str> {
    document_style
        .get("documentFormat")
        .and_then(|format| format.get("documentMode"))
        .or_else(|| document_style.get("documentMode"))
        .and_then(Value::as_str)
}

fn inherited_dimension_points(
    section_style: &Value,
    document_style: &Value,
    field: &str,
    description: &str,
) -> Result<f64> {
    let owner = if section_style.get(field).is_some() {
        section_style
    } else {
        document_style
    };
    dimension_points(owner, field, description)
}

fn dimension_points(owner: &Value, field: &str, description: &str) -> Result<f64> {
    let dimension = owner
        .get(field)
        .with_context(|| format!("Google Docs response omitted {description}"))?;
    let magnitude = dimension
        .get("magnitude")
        .and_then(Value::as_f64)
        .with_context(|| format!("Google Docs {description} omitted a numeric magnitude"))?;
    let unit = dimension
        .get("unit")
        .and_then(Value::as_str)
        .with_context(|| format!("Google Docs {description} omitted its unit"))?;
    if unit != "PT" {
        bail!("Google Docs {description} uses unsupported unit {unit:?}; expected PT");
    }
    if !magnitude.is_finite() || magnitude < 0.0 {
        bail!("Google Docs {description} must be finite and non-negative");
    }
    Ok(magnitude)
}

fn round_points(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}
