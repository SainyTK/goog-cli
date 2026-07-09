use std::future::Future;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::auth::config::Config;
use crate::auth::state::resource_key;
use crate::auth::unified_access::{AccessFuture, UnifiedAccess};
use crate::cli::{
    SlidesCommand, SlidesImageReplaceMethod, SlidesLineCategory, SlidesObjectCommand,
    SlidesPredefinedLayout, SlidesShapeType, SlidesSlideCommand, SlidesZOrderOperation,
};
use crate::slides::{
    batch_update_presentation, create_presentation, get_presentation,
    BatchUpdatePresentationOptions, CreatePresentationOptions, GetPresentationOptions, SlidesError,
};

pub fn run<S: AccountStore>(
    mut cmd: SlidesCommand,
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    output_json_by_default: bool,
    quiet: bool,
) -> Result<()> {
    cmd.normalize_presentation_id();
    match cmd {
        SlidesCommand::List {
            limit,
            all,
            folder,
            json,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(super::drive::run_slides_list_command_to(
                config,
                store,
                account_override,
                limit,
                all,
                folder,
                super::drive::should_emit_json(json, output_json_by_default),
                quiet,
                &mut std::io::stdout(),
                &mut std::io::stderr(),
                None,
            ))
        }
        SlidesCommand::Create { title } => {
            let client = AuthClient::from_config(config.clone(), store, account_override)?;
            run_with_runtime(run_create_to(&client, title, &mut std::io::stdout(), None))
        }
        SlidesCommand::Get {
            presentation_id,
            fields,
        } => run_with_runtime(run_get_unified_to(
            config,
            store,
            account_override,
            presentation_id,
            fields,
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::BatchUpdate {
            presentation_id,
            requests,
        } => {
            let mut stdin = std::io::stdin();
            run_with_runtime(run_batch_update_unified_to(
                config,
                store,
                account_override,
                presentation_id,
                requests,
                &mut stdin,
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        SlidesCommand::Slide {
            command:
                SlidesSlideCommand::Create {
                    presentation_id,
                    object_id,
                    insertion_index,
                    layout,
                },
        } => run_with_runtime(run_slide_create_unified_to(
            config,
            store,
            account_override,
            SlideCreateRequest {
                presentation_id,
                object_id,
                insertion_index,
                layout,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Slide {
            command:
                SlidesSlideCommand::Duplicate {
                    presentation_id,
                    page_id,
                    object_id,
                    insertion_index,
                },
        } => run_with_runtime(run_slide_duplicate_unified_to(
            config,
            store,
            account_override,
            SlideDuplicateRequest {
                presentation_id,
                page_id,
                object_id,
                insertion_index,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Slide {
            command:
                SlidesSlideCommand::Delete {
                    presentation_id,
                    page_id,
                },
        } => run_with_runtime(run_slide_delete_unified_to(
            config,
            store,
            account_override,
            SlideDeleteRequest {
                presentation_id,
                page_id,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Slide {
            command:
                SlidesSlideCommand::Background {
                    presentation_id,
                    page_id,
                    color,
                },
        } => run_with_runtime(run_slide_background_unified_to(
            config,
            store,
            account_override,
            SlideBackgroundRequest {
                presentation_id,
                page_id,
                color,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Object {
            command:
                SlidesObjectCommand::Move {
                    presentation_id,
                    object_id,
                    x,
                    y,
                    scale_x,
                    scale_y,
                },
        } => run_with_runtime(run_object_move_unified_to(
            config,
            store,
            account_override,
            ObjectMoveRequest {
                presentation_id,
                object_id,
                x,
                y,
                scale_x,
                scale_y,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Object {
            command:
                SlidesObjectCommand::Delete {
                    presentation_id,
                    object_id,
                },
        } => run_with_runtime(run_object_delete_unified_to(
            config,
            store,
            account_override,
            ObjectDeleteRequest {
                presentation_id,
                object_id,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Object {
            command:
                SlidesObjectCommand::Order {
                    presentation_id,
                    object_ids,
                    operation,
                },
        } => run_with_runtime(run_object_order_unified_to(
            config,
            store,
            account_override,
            ObjectOrderRequest {
                presentation_id,
                object_ids,
                operation,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Object {
            command:
                SlidesObjectCommand::Style {
                    presentation_id,
                    object_id,
                    fill_color,
                    outline_color,
                    outline_weight,
                },
        } => run_with_runtime(run_object_style_unified_to(
            config,
            store,
            account_override,
            ObjectStyleRequest {
                presentation_id,
                object_id,
                fill_color,
                outline_color,
                outline_weight,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Object {
            command:
                SlidesObjectCommand::TextStyle {
                    presentation_id,
                    object_id,
                    color,
                    font_family,
                    font_size,
                    bold,
                    italic,
                    underline,
                },
        } => run_with_runtime(run_object_text_style_unified_to(
            config,
            store,
            account_override,
            ObjectTextStyleRequest {
                presentation_id,
                object_id,
                color,
                font_family,
                font_size,
                bold,
                italic,
                underline,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Object {
            command:
                SlidesObjectCommand::InsertText {
                    presentation_id,
                    object_id,
                    text,
                    index,
                },
        } => run_with_runtime(run_object_insert_text_unified_to(
            config,
            store,
            account_override,
            ObjectInsertTextRequest {
                presentation_id,
                object_id,
                text,
                index,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Object {
            command:
                SlidesObjectCommand::AltText {
                    presentation_id,
                    object_id,
                    title,
                    description,
                },
        } => run_with_runtime(run_object_alt_text_unified_to(
            config,
            store,
            account_override,
            ObjectAltTextRequest {
                presentation_id,
                object_id,
                title,
                description,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Object {
            command:
                SlidesObjectCommand::ReplaceImage {
                    presentation_id,
                    image_id,
                    url,
                    method,
                },
        } => run_with_runtime(run_object_replace_image_unified_to(
            config,
            store,
            account_override,
            ObjectReplaceImageRequest {
                presentation_id,
                image_id,
                url,
                method,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::TextBox {
            presentation_id,
            page_id,
            text,
            object_id,
            x,
            y,
            width,
            height,
        } => run_with_runtime(run_text_box_unified_to(
            config,
            store,
            account_override,
            TextBoxRequest {
                presentation_id,
                page_id,
                text,
                object_id,
                x,
                y,
                width,
                height,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Image {
            presentation_id,
            page_id,
            url,
            object_id,
            x,
            y,
            width,
            height,
        } => run_with_runtime(run_image_unified_to(
            config,
            store,
            account_override,
            ImageRequest {
                presentation_id,
                page_id,
                url,
                object_id,
                x,
                y,
                width,
                height,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Table {
            presentation_id,
            page_id,
            rows,
            columns,
            object_id,
            x,
            y,
            width,
            height,
        } => run_with_runtime(run_table_unified_to(
            config,
            store,
            account_override,
            TableRequest {
                presentation_id,
                page_id,
                rows,
                columns,
                object_id,
                x,
                y,
                width,
                height,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::TableFill {
            presentation_id,
            table_id,
            rows,
            delimiter,
            start_row,
            start_column,
        } => run_with_runtime(run_table_fill_unified_to(
            config,
            store,
            account_override,
            TableFillRequest {
                presentation_id,
                table_id,
                rows,
                delimiter,
                start_row,
                start_column,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Shape {
            presentation_id,
            page_id,
            shape_type,
            object_id,
            x,
            y,
            width,
            height,
        } => run_with_runtime(run_shape_unified_to(
            config,
            store,
            account_override,
            ShapeRequest {
                presentation_id,
                page_id,
                shape_type,
                object_id,
                x,
                y,
                width,
                height,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::Line {
            presentation_id,
            page_id,
            category,
            object_id,
            x,
            y,
            width,
            height,
        } => run_with_runtime(run_line_unified_to(
            config,
            store,
            account_override,
            LineRequest {
                presentation_id,
                page_id,
                category,
                object_id,
                x,
                y,
                width,
                height,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
        SlidesCommand::ReplaceText {
            presentation_id,
            find,
            replacement,
            match_case,
            page_ids,
        } => run_with_runtime(run_replace_text_unified_to(
            config,
            store,
            account_override,
            ReplaceTextRequest {
                presentation_id,
                find,
                replacement,
                match_case,
                page_ids,
            },
            &mut std::io::stdout(),
            None,
            None,
        )),
    }
}

fn run_with_runtime(future: impl Future<Output = Result<()>>) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("failed to start async runtime")?;
    runtime.block_on(future)
}

pub(super) async fn run_create_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    title: String,
    out: &mut impl Write,
    presentations_url: Option<&str>,
) -> Result<()> {
    let mut options = CreatePresentationOptions::new(title);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let presentation = create_presentation(client, &options)
        .await
        .context("failed to create Google Slides presentation")?;
    let presentation_id = presentation
        .get("presentationId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    writeln!(
        out,
        "{presentation_id}\thttps://docs.google.com/presentation/d/{presentation_id}/edit"
    )
    .context("failed to write output")
}

pub(super) async fn run_get_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    presentation_id: String,
    fields: Option<String>,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let mut options = GetPresentationOptions::new(presentation_id.clone());
    if let Some(fields) = fields {
        options = options.with_fields(fields);
    }
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let presentation = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::Get(&options),
        state_path,
    )
    .await
    .context("failed to read Google Slides presentation")?;

    write_json_line(
        out,
        &presentation,
        "failed to serialize Slides presentation",
    )
}

pub(super) async fn run_batch_update_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    presentation_id: String,
    requests: String,
    input: &mut impl Read,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let request_body = read_request_body(&requests, input)?;
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to apply Google Slides Batch Update")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides Batch Update response",
    )
}

#[derive(Debug, Clone)]
pub(super) struct SlideCreateRequest {
    pub presentation_id: String,
    pub object_id: Option<String>,
    pub insertion_index: Option<u32>,
    pub layout: SlidesPredefinedLayout,
}

pub(super) async fn run_slide_create_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: SlideCreateRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_slide_create_batch_update(request);
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to create Google Slides slide")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides slide create response",
    )
}

fn build_slide_create_batch_update(request: SlideCreateRequest) -> serde_json::Value {
    let mut create_slide = serde_json::json!({
        "slideLayoutReference": {
            "predefinedLayout": request.layout.api_value()
        }
    });

    if let Some(object_id) = request.object_id {
        create_slide["objectId"] = serde_json::Value::String(object_id);
    }

    if let Some(insertion_index) = request.insertion_index {
        create_slide["insertionIndex"] = serde_json::json!(insertion_index);
    }

    serde_json::json!({
        "requests": [
            {
                "createSlide": create_slide
            }
        ]
    })
}

#[derive(Debug, Clone)]
pub(super) struct SlideDuplicateRequest {
    pub presentation_id: String,
    pub page_id: String,
    pub object_id: Option<String>,
    pub insertion_index: Option<u32>,
}

pub(super) async fn run_slide_duplicate_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: SlideDuplicateRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_slide_duplicate_batch_update(request);
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to duplicate Google Slides slide")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides slide duplicate response",
    )
}

fn build_slide_duplicate_batch_update(request: SlideDuplicateRequest) -> serde_json::Value {
    let mut requests = vec![{
        let mut duplicate_object = serde_json::json!({
            "objectId": request.page_id
        });

        if let Some(object_id) = &request.object_id {
            duplicate_object["objectIds"] = serde_json::json!({
                request.page_id.clone(): object_id
            });
        }

        serde_json::json!({
            "duplicateObject": duplicate_object
        })
    }];

    if let (Some(object_id), Some(insertion_index)) = (&request.object_id, request.insertion_index)
    {
        requests.push(serde_json::json!({
            "updateSlidesPosition": {
                "slideObjectIds": [object_id],
                "insertionIndex": insertion_index
            }
        }));
    }

    serde_json::json!({
        "requests": requests
    })
}

#[derive(Debug, Clone)]
pub(super) struct SlideDeleteRequest {
    pub presentation_id: String,
    pub page_id: String,
}

pub(super) async fn run_slide_delete_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: SlideDeleteRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_slide_delete_batch_update(request);
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to delete Google Slides slide")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides slide delete response",
    )
}

fn build_slide_delete_batch_update(request: SlideDeleteRequest) -> serde_json::Value {
    build_delete_object_batch_update(request.page_id)
}

#[derive(Debug, Clone)]
pub(super) struct SlideBackgroundRequest {
    pub presentation_id: String,
    pub page_id: String,
    pub color: String,
}

pub(super) async fn run_slide_background_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: SlideBackgroundRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_slide_background_batch_update(request)?;
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to set Google Slides slide background")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides slide background response",
    )
}

pub(super) fn build_slide_background_batch_update(
    request: SlideBackgroundRequest,
) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "requests": [
            {
                "updatePageProperties": {
                    "objectId": request.page_id,
                    "pageProperties": {
                        "pageBackgroundFill": {
                            "solidFill": {
                                "color": {
                                    "rgbColor": parse_hex_rgb_color(&request.color)?
                                }
                            }
                        }
                    },
                    "fields": "pageBackgroundFill.solidFill.color"
                }
            }
        ]
    }))
}

#[derive(Debug, Clone)]
pub(super) struct ObjectMoveRequest {
    pub presentation_id: String,
    pub object_id: String,
    pub x: f64,
    pub y: f64,
    pub scale_x: f64,
    pub scale_y: f64,
}

pub(super) async fn run_object_move_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: ObjectMoveRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_object_move_batch_update(request);
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to move Google Slides object")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides object move response",
    )
}

fn build_object_move_batch_update(request: ObjectMoveRequest) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "updatePageElementTransform": {
                    "objectId": request.object_id,
                    "applyMode": "ABSOLUTE",
                    "transform": {
                        "scaleX": request.scale_x,
                        "scaleY": request.scale_y,
                        "translateX": request.x,
                        "translateY": request.y,
                        "unit": "PT"
                    }
                }
            }
        ]
    })
}

#[derive(Debug, Clone)]
pub(super) struct ObjectOrderRequest {
    pub presentation_id: String,
    pub object_ids: Vec<String>,
    pub operation: SlidesZOrderOperation,
}

pub(super) async fn run_object_order_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: ObjectOrderRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_object_order_batch_update(request);
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to arrange Google Slides objects")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides object order response",
    )
}

fn build_object_order_batch_update(request: ObjectOrderRequest) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "updatePageElementsZOrder": {
                    "pageElementObjectIds": request.object_ids,
                    "operation": request.operation.api_value()
                }
            }
        ]
    })
}

#[derive(Debug, Clone)]
pub(super) struct ObjectStyleRequest {
    pub presentation_id: String,
    pub object_id: String,
    pub fill_color: Option<String>,
    pub outline_color: Option<String>,
    pub outline_weight: Option<f64>,
}

pub(super) async fn run_object_style_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: ObjectStyleRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_object_style_batch_update(request)?;
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to style Google Slides object")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides object style response",
    )
}

pub(super) fn build_object_style_batch_update(
    request: ObjectStyleRequest,
) -> Result<serde_json::Value> {
    if request.fill_color.is_none()
        && request.outline_color.is_none()
        && request.outline_weight.is_none()
    {
        bail!("at least one style flag is required");
    }

    let mut shape_properties = serde_json::Map::new();
    let mut fields = Vec::new();

    if let Some(color) = request.fill_color {
        shape_properties.insert(
            "shapeBackgroundFill".into(),
            serde_json::json!({
                "solidFill": {
                    "color": {
                        "rgbColor": parse_hex_rgb_color(&color)?
                    }
                }
            }),
        );
        fields.push("shapeBackgroundFill.solidFill.color");
    }

    let mut outline = serde_json::Map::new();
    if let Some(color) = request.outline_color {
        outline.insert(
            "outlineFill".into(),
            serde_json::json!({
                "solidFill": {
                    "color": {
                        "rgbColor": parse_hex_rgb_color(&color)?
                    }
                }
            }),
        );
        fields.push("outline.outlineFill.solidFill.color");
    }

    if let Some(weight) = request.outline_weight {
        outline.insert(
            "weight".into(),
            serde_json::json!({
                "magnitude": weight,
                "unit": "PT"
            }),
        );
        fields.push("outline.weight");
    }

    if !outline.is_empty() {
        shape_properties.insert("outline".into(), serde_json::Value::Object(outline));
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "updateShapeProperties": {
                    "objectId": request.object_id,
                    "shapeProperties": shape_properties,
                    "fields": fields.join(",")
                }
            }
        ]
    }))
}

#[derive(Debug, Clone)]
pub(super) struct ObjectTextStyleRequest {
    pub presentation_id: String,
    pub object_id: String,
    pub color: Option<String>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
}

pub(super) async fn run_object_text_style_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: ObjectTextStyleRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_object_text_style_batch_update(request)?;
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to style Google Slides object text")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides object text style response",
    )
}

pub(super) fn build_object_text_style_batch_update(
    request: ObjectTextStyleRequest,
) -> Result<serde_json::Value> {
    if request.color.is_none()
        && request.font_family.is_none()
        && request.font_size.is_none()
        && request.bold.is_none()
        && request.italic.is_none()
        && request.underline.is_none()
    {
        bail!("at least one text style flag is required");
    }

    let mut style = serde_json::Map::new();
    let mut fields = Vec::new();

    if let Some(color) = request.color {
        style.insert(
            "foregroundColor".into(),
            serde_json::json!({
                "opaqueColor": {
                    "rgbColor": parse_hex_rgb_color(&color)?
                }
            }),
        );
        fields.push("foregroundColor");
    }

    if let Some(font_family) = request.font_family {
        style.insert("fontFamily".into(), serde_json::Value::String(font_family));
        fields.push("fontFamily");
    }

    if let Some(font_size) = request.font_size {
        style.insert(
            "fontSize".into(),
            serde_json::json!({
                "magnitude": font_size,
                "unit": "PT"
            }),
        );
        fields.push("fontSize");
    }

    if let Some(bold) = request.bold {
        style.insert("bold".into(), serde_json::Value::Bool(bold));
        fields.push("bold");
    }

    if let Some(italic) = request.italic {
        style.insert("italic".into(), serde_json::Value::Bool(italic));
        fields.push("italic");
    }

    if let Some(underline) = request.underline {
        style.insert("underline".into(), serde_json::Value::Bool(underline));
        fields.push("underline");
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "updateTextStyle": {
                    "objectId": request.object_id,
                    "style": style,
                    "textRange": {
                        "type": "ALL"
                    },
                    "fields": fields.join(",")
                }
            }
        ]
    }))
}

#[derive(Debug, Clone)]
pub(super) struct ObjectInsertTextRequest {
    pub presentation_id: String,
    pub object_id: String,
    pub text: String,
    pub index: u32,
}

pub(super) async fn run_object_insert_text_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: ObjectInsertTextRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_object_insert_text_batch_update(request)?;
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to insert text into Google Slides object")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides object text insertion response",
    )
}

pub(super) fn build_object_insert_text_batch_update(
    request: ObjectInsertTextRequest,
) -> Result<serde_json::Value> {
    if request.text.is_empty() {
        bail!("--text must not be empty");
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "insertText": {
                    "objectId": request.object_id,
                    "text": request.text,
                    "insertionIndex": request.index
                }
            }
        ]
    }))
}

#[derive(Debug, Clone)]
pub(super) struct ObjectAltTextRequest {
    pub presentation_id: String,
    pub object_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

pub(super) async fn run_object_alt_text_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: ObjectAltTextRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_object_alt_text_batch_update(request)?;
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to set Google Slides object alt text")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides object alt text response",
    )
}

pub(super) fn build_object_alt_text_batch_update(
    request: ObjectAltTextRequest,
) -> Result<serde_json::Value> {
    if request.title.is_none() && request.description.is_none() {
        bail!("at least one alt text flag is required");
    }

    let mut update = serde_json::Map::new();
    update.insert(
        "objectId".into(),
        serde_json::Value::String(request.object_id),
    );

    if let Some(title) = request.title {
        update.insert("title".into(), serde_json::Value::String(title));
    }

    if let Some(description) = request.description {
        update.insert("description".into(), serde_json::Value::String(description));
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "updatePageElementAltText": update
            }
        ]
    }))
}

#[derive(Debug, Clone)]
pub(super) struct ObjectReplaceImageRequest {
    pub presentation_id: String,
    pub image_id: String,
    pub url: String,
    pub method: SlidesImageReplaceMethod,
}

pub(super) async fn run_object_replace_image_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: ObjectReplaceImageRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_object_replace_image_batch_update(request);
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to replace Google Slides image")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides image replacement response",
    )
}

fn build_object_replace_image_batch_update(
    request: ObjectReplaceImageRequest,
) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "replaceImage": {
                    "imageObjectId": request.image_id,
                    "url": request.url,
                    "imageReplaceMethod": request.method.as_api_value()
                }
            }
        ]
    })
}

fn parse_hex_rgb_color(color: &str) -> Result<serde_json::Value> {
    let hex = color.strip_prefix('#').unwrap_or(color);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("color must be a 6-digit hex value like #1a73e8");
    }

    let red = u8::from_str_radix(&hex[0..2], 16).context("failed to parse red hex channel")?;
    let green = u8::from_str_radix(&hex[2..4], 16).context("failed to parse green hex channel")?;
    let blue = u8::from_str_radix(&hex[4..6], 16).context("failed to parse blue hex channel")?;

    Ok(serde_json::json!({
        "red": f64::from(red) / 255.0,
        "green": f64::from(green) / 255.0,
        "blue": f64::from(blue) / 255.0
    }))
}

#[derive(Debug, Clone)]
pub(super) struct ObjectDeleteRequest {
    pub presentation_id: String,
    pub object_id: String,
}

pub(super) async fn run_object_delete_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: ObjectDeleteRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_object_delete_batch_update(request);
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to delete Google Slides object")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides object delete response",
    )
}

fn build_object_delete_batch_update(request: ObjectDeleteRequest) -> serde_json::Value {
    build_delete_object_batch_update(request.object_id)
}

fn build_delete_object_batch_update(object_id: String) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "deleteObject": {
                    "objectId": object_id
                }
            }
        ]
    })
}

#[derive(Debug, Clone)]
pub(super) struct TextBoxRequest {
    pub presentation_id: String,
    pub page_id: String,
    pub text: String,
    pub object_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) async fn run_text_box_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: TextBoxRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_text_box_batch_update(request);
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to add Google Slides text box")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides text box response",
    )
}

fn build_text_box_batch_update(request: TextBoxRequest) -> serde_json::Value {
    let object_id = request
        .object_id
        .unwrap_or_else(generated_text_box_object_id);
    serde_json::json!({
        "requests": [
            {
                "createShape": {
                    "objectId": object_id,
                    "shapeType": "TEXT_BOX",
                    "elementProperties": {
                        "pageObjectId": request.page_id,
                        "size": {
                            "width": {
                                "magnitude": request.width,
                                "unit": "PT"
                            },
                            "height": {
                                "magnitude": request.height,
                                "unit": "PT"
                            }
                        },
                        "transform": {
                            "scaleX": 1.0,
                            "scaleY": 1.0,
                            "translateX": request.x,
                            "translateY": request.y,
                            "unit": "PT"
                        }
                    }
                }
            },
            {
                "insertText": {
                    "objectId": object_id,
                    "insertionIndex": 0,
                    "text": request.text
                }
            }
        ]
    })
}

fn generated_text_box_object_id() -> String {
    format!("goog_text_box_{}", chrono::Utc::now().timestamp_millis())
}

#[derive(Debug, Clone)]
pub(super) struct ImageRequest {
    pub presentation_id: String,
    pub page_id: String,
    pub url: String,
    pub object_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) async fn run_image_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: ImageRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_image_batch_update(request);
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to add Google Slides image")?;

    write_json_line(out, &response, "failed to serialize Slides image response")
}

fn build_image_batch_update(request: ImageRequest) -> serde_json::Value {
    let object_id = request.object_id.unwrap_or_else(generated_image_object_id);
    serde_json::json!({
        "requests": [
            {
                "createImage": {
                    "objectId": object_id,
                    "url": request.url,
                    "elementProperties": {
                        "pageObjectId": request.page_id,
                        "size": {
                            "width": {
                                "magnitude": request.width,
                                "unit": "PT"
                            },
                            "height": {
                                "magnitude": request.height,
                                "unit": "PT"
                            }
                        },
                        "transform": {
                            "scaleX": 1.0,
                            "scaleY": 1.0,
                            "translateX": request.x,
                            "translateY": request.y,
                            "unit": "PT"
                        }
                    }
                }
            }
        ]
    })
}

fn generated_image_object_id() -> String {
    format!("goog_image_{}", chrono::Utc::now().timestamp_millis())
}

#[derive(Debug, Clone)]
pub(super) struct TableRequest {
    pub presentation_id: String,
    pub page_id: String,
    pub rows: u32,
    pub columns: u32,
    pub object_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) async fn run_table_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: TableRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_table_batch_update(request)?;
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to add Google Slides table")?;

    write_json_line(out, &response, "failed to serialize Slides table response")
}

fn build_table_batch_update(request: TableRequest) -> Result<serde_json::Value> {
    if request.rows == 0 {
        anyhow::bail!("slides table --rows must be greater than zero");
    }
    if request.columns == 0 {
        anyhow::bail!("slides table --columns must be greater than zero");
    }

    let object_id = request.object_id.unwrap_or_else(generated_table_object_id);
    Ok(serde_json::json!({
        "requests": [
            {
                "createTable": {
                    "objectId": object_id,
                    "rows": request.rows,
                    "columns": request.columns,
                    "elementProperties": {
                        "pageObjectId": request.page_id,
                        "size": {
                            "width": {
                                "magnitude": request.width,
                                "unit": "PT"
                            },
                            "height": {
                                "magnitude": request.height,
                                "unit": "PT"
                            }
                        },
                        "transform": {
                            "scaleX": 1.0,
                            "scaleY": 1.0,
                            "translateX": request.x,
                            "translateY": request.y,
                            "unit": "PT"
                        }
                    }
                }
            }
        ]
    }))
}

fn generated_table_object_id() -> String {
    format!("goog_table_{}", chrono::Utc::now().timestamp_millis())
}

#[derive(Debug, Clone)]
pub(super) struct TableFillRequest {
    pub presentation_id: String,
    pub table_id: String,
    pub rows: Vec<String>,
    pub delimiter: String,
    pub start_row: u32,
    pub start_column: u32,
}

pub(super) async fn run_table_fill_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: TableFillRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_table_fill_batch_update(request)?;
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to fill Google Slides table")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides table fill response",
    )
}

pub(super) fn build_table_fill_batch_update(
    request: TableFillRequest,
) -> Result<serde_json::Value> {
    if request.delimiter.is_empty() {
        bail!("slides table-fill --delimiter must not be empty");
    }

    let delimiter = request.delimiter.as_str();
    let mut requests = Vec::new();

    for (row_offset, row) in request.rows.iter().enumerate() {
        for (column_offset, cell_text) in row.split(delimiter).enumerate() {
            if cell_text.is_empty() {
                continue;
            }

            requests.push(serde_json::json!({
                "insertText": {
                    "objectId": &request.table_id,
                    "cellLocation": {
                        "rowIndex": request.start_row + row_offset as u32,
                        "columnIndex": request.start_column + column_offset as u32
                    },
                    "insertionIndex": 0,
                    "text": cell_text
                }
            }));
        }
    }

    if requests.is_empty() {
        bail!("slides table-fill requires at least one non-empty cell");
    }

    Ok(serde_json::json!({
        "requests": requests
    }))
}

#[derive(Debug, Clone)]
pub(super) struct ShapeRequest {
    pub presentation_id: String,
    pub page_id: String,
    pub shape_type: SlidesShapeType,
    pub object_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) async fn run_shape_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: ShapeRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_shape_batch_update(request);
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to add Google Slides shape")?;

    write_json_line(out, &response, "failed to serialize Slides shape response")
}

fn build_shape_batch_update(request: ShapeRequest) -> serde_json::Value {
    let object_id = request.object_id.unwrap_or_else(generated_shape_object_id);
    serde_json::json!({
        "requests": [
            {
                "createShape": {
                    "objectId": object_id,
                    "shapeType": request.shape_type.api_value(),
                    "elementProperties": {
                        "pageObjectId": request.page_id,
                        "size": {
                            "width": {
                                "magnitude": request.width,
                                "unit": "PT"
                            },
                            "height": {
                                "magnitude": request.height,
                                "unit": "PT"
                            }
                        },
                        "transform": {
                            "scaleX": 1.0,
                            "scaleY": 1.0,
                            "translateX": request.x,
                            "translateY": request.y,
                            "unit": "PT"
                        }
                    }
                }
            }
        ]
    })
}

fn generated_shape_object_id() -> String {
    format!("goog_shape_{}", chrono::Utc::now().timestamp_millis())
}

#[derive(Debug, Clone)]
pub(super) struct LineRequest {
    pub presentation_id: String,
    pub page_id: String,
    pub category: SlidesLineCategory,
    pub object_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) async fn run_line_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: LineRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_line_batch_update(request);
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to add Google Slides line")?;

    write_json_line(out, &response, "failed to serialize Slides line response")
}

fn build_line_batch_update(request: LineRequest) -> serde_json::Value {
    let object_id = request.object_id.unwrap_or_else(generated_line_object_id);
    serde_json::json!({
        "requests": [
            {
                "createLine": {
                    "objectId": object_id,
                    "category": request.category.api_value(),
                    "elementProperties": {
                        "pageObjectId": request.page_id,
                        "size": {
                            "width": {
                                "magnitude": request.width,
                                "unit": "PT"
                            },
                            "height": {
                                "magnitude": request.height,
                                "unit": "PT"
                            }
                        },
                        "transform": {
                            "scaleX": 1.0,
                            "scaleY": 1.0,
                            "translateX": request.x,
                            "translateY": request.y,
                            "unit": "PT"
                        }
                    }
                }
            }
        ]
    })
}

fn generated_line_object_id() -> String {
    format!("goog_line_{}", chrono::Utc::now().timestamp_millis())
}

#[derive(Debug, Clone)]
pub(super) struct ReplaceTextRequest {
    pub presentation_id: String,
    pub find: String,
    pub replacement: String,
    pub match_case: bool,
    pub page_ids: Vec<String>,
}

pub(super) async fn run_replace_text_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    request: ReplaceTextRequest,
    out: &mut impl Write,
    presentations_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let presentation_id = request.presentation_id.clone();
    let request_body = build_replace_text_batch_update(request)?;
    let mut options = BatchUpdatePresentationOptions::new(presentation_id.clone(), request_body);
    if let Some(presentations_url) = presentations_url {
        options = options.with_presentations_url(presentations_url);
    }

    let target_resource_key = resource_key("slides", &presentation_id);
    let response = run_with_slides_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        SlidesAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to replace Google Slides text")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Slides replace text response",
    )
}

fn build_replace_text_batch_update(request: ReplaceTextRequest) -> Result<serde_json::Value> {
    if request.find.is_empty() {
        anyhow::bail!("slides replace-text --find must not be empty");
    }

    let mut replace_all_text = serde_json::json!({
        "containsText": {
            "text": request.find,
            "matchCase": request.match_case
        },
        "replaceText": request.replacement
    });

    if !request.page_ids.is_empty() {
        replace_all_text["pageObjectIds"] = serde_json::json!(request.page_ids);
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "replaceAllText": replace_all_text
            }
        ]
    }))
}

enum SlidesAccessAttempt<'a> {
    Get(&'a GetPresentationOptions),
    BatchUpdate(&'a BatchUpdatePresentationOptions),
}

async fn run_with_slides_unified_access<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    attempt: SlidesAccessAttempt<'_>,
    state_path: Option<&Path>,
) -> Result<serde_json::Value, SlidesError> {
    UnifiedAccess::run(
        config,
        account_override,
        target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, serde_json::Value, SlidesError> {
            Box::pin(run_slides_access_as_account(
                config, store, &attempt, account,
            ))
        },
        is_target_access_failure,
    )
    .await
}

async fn run_slides_access_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    attempt: &SlidesAccessAttempt<'_>,
    account: String,
) -> Result<serde_json::Value, SlidesError> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))
        .map_err(SlidesError::Auth)?;
    match attempt {
        SlidesAccessAttempt::Get(options) => get_presentation(&client, options).await,
        SlidesAccessAttempt::BatchUpdate(options) => {
            batch_update_presentation(&client, options).await
        }
    }
}

fn is_target_access_failure(err: &SlidesError) -> bool {
    matches!(err, SlidesError::NotFound | SlidesError::PermissionDenied)
}

fn read_request_body(path_or_stdin: &str, input: &mut impl Read) -> Result<serde_json::Value> {
    let (body, request_source) = if path_or_stdin == "-" {
        let mut body = String::new();
        input
            .read_to_string(&mut body)
            .context("failed to read Google Slides Batch Update request body from stdin")?;
        (body, "stdin".to_string())
    } else {
        let body = std::fs::read_to_string(path_or_stdin).with_context(|| {
            format!("failed to read Google Slides Batch Update request body: {path_or_stdin}")
        })?;
        (body, path_or_stdin.to_string())
    };

    serde_json::from_str(&body).with_context(|| {
        format!("failed to parse Google Slides Batch Update request body from {request_source}")
    })
}

fn write_json_line(out: &mut impl Write, value: &serde_json::Value, context: &str) -> Result<()> {
    serde_json::to_writer(&mut *out, value).with_context(|| context.to_string())?;
    writeln!(out).context("failed to write output")
}
