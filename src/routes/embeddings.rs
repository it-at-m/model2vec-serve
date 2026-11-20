//! OpenAI-compatible `/v1/embeddings` endpoint.

use crate::{
    errors::AppError,
    routes::dto::{
        EmbeddingObject, EmbeddingRequest, EmbeddingResponse, EmbeddingVector, ErrorResponse,
        ModelsListResponse, OpenAiModelInfo, Usage,
    },
    state::AppState,
    telemetry::RequestModelId,
};
use axum::{
    Extension, Json,
    extract::State,
    http::{HeaderMap, header::HeaderName},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::sync::Arc;

/// Name of the optional per-request normalization header.
///
/// HTTP header names are case-insensitive; this is the canonical spelling
/// used in documentation.
pub const HEADER_NAME: HeaderName = HeaderName::from_static("x-normalize");

/// Encode a vector of floats as base64.
fn encode_base64(values: &[f32]) -> String {
    let bytes: Vec<u8> = values.iter().flat_map(|f| f.to_le_bytes()).collect();
    STANDARD.encode(bytes)
}

/// Parse the `X-Normalize` request header values.
///
/// `values` holds every raw string value sent for the header, converted with
/// [`axum::http::HeaderValue::to_str`]; values that are not valid visible
/// ASCII never reach this function and are rejected by the caller.
///
/// Absent (`values` empty) parses to `Ok(None)` — the model's configured
/// normalization default applies. A single `true` or `false` in any letter
/// case parses to `Ok(Some(_))`. Any other value, or more than one value
/// (duplicate header), is an error.
///
/// # Errors
///
/// Returns a message naming the header and the accepted values when a value
/// is unrecognized, empty, or duplicated.
pub fn parse_normalize_header(values: &[&str]) -> Result<Option<bool>, String> {
    match values {
        [] => Ok(None),
        [value] => parse_single_normalize_value(value),
        _ => Err(format!(
            "the X-Normalize header was sent {} times; send it at most once with 'true' or 'false'",
            values.len()
        )),
    }
}

/// Parse one raw `X-Normalize` value, accepting `true`/`false` in any letter
/// case.
///
/// # Errors
///
/// Returns a message naming the header and the accepted values for empty or
/// unrecognized values.
fn parse_single_normalize_value(value: &str) -> Result<Option<bool>, String> {
    if value.eq_ignore_ascii_case("true") {
        Ok(Some(true))
    } else if value.eq_ignore_ascii_case("false") {
        Ok(Some(false))
    } else if value.is_empty() {
        Err("the X-Normalize header must not be empty; use 'true' or 'false'".to_string())
    } else {
        Err(format!(
            "invalid X-Normalize value {value:?}; use 'true' or 'false'"
        ))
    }
}

/// OpenAI-compatible embeddings endpoint.
///
/// # Errors
///
/// Returns `AppError::BadRequest` for invalid input, `AppError::Unauthorized`
/// when authentication is enabled and fails, or `AppError::Internal` if model
/// inference fails.
#[utoipa::path(
    post,
    path = "/v1/embeddings",
    tag = "embeddings",
    params(
        (
            "X-Normalize" = String,
            Header,
            description = "Service-specific extension: optional per-request normalization control. \
                           'true' forces L2-normalized embeddings, 'false' returns raw mean-pooled \
                           vectors; absent keeps the model's configured default. Invalid or \
                           duplicate values are rejected with 400.",
            example = "false"
        ),
    ),
    request_body = EmbeddingRequest,
    responses(
        (status = 200, description = "Embeddings generated", body = EmbeddingResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Model inference error", body = ErrorResponse)
    )
)]
pub async fn create_embeddings(
    State(state): State<Arc<AppState>>,
    Extension(model_id_ext): Extension<RequestModelId>,
    headers: HeaderMap,
    Json(request): Json<EmbeddingRequest>,
) -> Result<Json<EmbeddingResponse>, AppError> {
    validate_request(&request, &state)?;

    let normalize = parse_normalize_values(&headers)?;

    let model_id = request
        .model
        .as_deref()
        .unwrap_or_else(|| state.registry.default_model_id());
    let loaded = state.registry.resolve(Some(model_id))?;

    let inputs = request.input.as_strings();
    let embeddings = loaded
        .model
        .encode(&inputs, loaded.max_input_length, inputs.len(), normalize);

    let data: Vec<EmbeddingObject> = embeddings
        .into_iter()
        .enumerate()
        .map(|(index, vector)| {
            let embedding = if request.encoding_format == "base64" {
                EmbeddingVector::Base64(encode_base64(&vector))
            } else {
                EmbeddingVector::Float(vector)
            };
            EmbeddingObject {
                object: "embedding",
                index,
                embedding,
            }
        })
        .collect();

    let prompt_tokens = inputs.iter().map(String::len).sum::<usize>() / 4;
    let response = EmbeddingResponse {
        object: "list",
        data,
        model: loaded.model_id.clone(),
        usage: Usage {
            prompt_tokens,
            total_tokens: prompt_tokens,
        },
    };

    model_id_ext.set(loaded.model_id.clone());

    Ok(Json(response))
}

/// Collect and parse all `X-Normalize` header values from the request.
///
/// Header values that are not valid visible ASCII are rejected the same way
/// as unrecognized values.
///
/// # Errors
///
/// Returns [`AppError::BadRequest`] when the header is duplicated, empty,
/// unrecognized, or not valid header-value ASCII.
fn parse_normalize_values(headers: &HeaderMap) -> Result<Option<bool>, AppError> {
    let values = headers
        .get_all(HEADER_NAME)
        .iter()
        .map(|value| {
            value.to_str().map_err(|_| {
                "the X-Normalize header value is not valid ASCII; use 'true' or 'false'".to_string()
            })
        })
        .collect::<Result<Vec<&str>, String>>()
        .map_err(AppError::BadRequest)?;

    parse_normalize_header(&values).map_err(AppError::BadRequest)
}

fn validate_request(request: &EmbeddingRequest, state: &AppState) -> Result<(), AppError> {
    if request.input.is_empty() {
        return Err(AppError::BadRequest("input cannot be empty".to_string()));
    }

    if request.input.len() > state.config.max_batch_size {
        return Err(AppError::BadRequest(format!(
            "batch size exceeds maximum of {}",
            state.config.max_batch_size
        )));
    }

    if !matches!(request.encoding_format.as_str(), "float" | "base64") {
        return Err(AppError::BadRequest(
            "encoding_format must be 'float' or 'base64'".to_string(),
        ));
    }

    if let Some(ref model) = request.model
        && state.registry.get(model).is_none()
    {
        return Err(AppError::ModelNotFound(format!(
            "model '{model}' is not loaded"
        )));
    }

    Ok(())
}

/// OpenAI-compatible models list endpoint.
///
/// # Errors
///
/// Returns `AppError::Unauthorized` when authentication is enabled and fails.
#[utoipa::path(
    get,
    path = "/v1/models",
    tag = "embeddings",
    responses(
        (status = 200, description = "Models listed", body = ModelsListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    )
)]
pub async fn list_models(State(state): State<Arc<AppState>>) -> Json<ModelsListResponse> {
    const CREATED: i64 = 1_686_935_002;
    let owner = state.config.model_owner.clone();
    let data = state
        .registry
        .iter()
        .map(|model| OpenAiModelInfo {
            id: model.model_id.clone(),
            object: "model",
            created: CREATED,
            owned_by: owner.clone(),
        })
        .collect();

    Json(ModelsListResponse {
        object: "list",
        data,
    })
}
