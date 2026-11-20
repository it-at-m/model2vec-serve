#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

mod common;

// Contract tests for the OpenAI-compatible embeddings endpoint.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::test_app;
use http_body_util::BodyExt;
use model2vec_serve::routes::embeddings::parse_normalize_header;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Query `/v1/models` and return the id of the first (and usually only) loaded model.
async fn fixture_model_id(app: axum::Router) -> String {
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&body).unwrap();

    value["data"][0]["id"]
        .as_str()
        .expect("models list should contain a model id")
        .to_string()
}

#[tokio::test]
async fn openai_models_lists_loaded_models() {
    let app = test_app(None).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value["object"], "list");
    assert!(value["data"].is_array());

    let data = value["data"].as_array().unwrap();
    assert_ne!(data.len(), 0);

    let model = &data[0];
    assert!(model["id"].is_string());
    assert_eq!(model["object"], "model");
    assert!(model["created"].is_number());
    assert!(model["owned_by"].is_string());
}

#[tokio::test]
async fn openai_embeddings_with_model_returns_requested_model() {
    let app = test_app(None).await;
    let model_id = fixture_model_id(app.clone()).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/embeddings")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"input": "Hello world", "model": model_id}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value["model"], model_id);
}

#[tokio::test]
async fn openai_embeddings_without_model_uses_default() {
    let app = test_app(None).await;
    let default_model_id = fixture_model_id(app.clone()).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/embeddings")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"input": "Hello world"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value["model"], default_model_id);
}

#[tokio::test]
async fn openai_embeddings_with_unknown_model_returns_error() {
    let app = test_app(None).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/embeddings")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"input": "Hello world", "model": "definitely-not-loaded"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value["error"], "model_not_found");
}

#[tokio::test]
async fn openai_embeddings_supports_batch_input() {
    let app = test_app(None).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/embeddings")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"input": ["a", "b"]}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value["data"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn openai_embeddings_rejects_empty_input() {
    let app = test_app(None).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/embeddings")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"input": []}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// `X-Normalize` header parser unit tests (spec 009, data-model.md).

#[test]
fn normalize_header_absent_parses_to_none() {
    assert_eq!(parse_normalize_header(&[]), Ok(None));
}

#[test]
fn normalize_header_true_false_accepted_case_insensitively() {
    assert_eq!(parse_normalize_header(&["true"]), Ok(Some(true)));
    assert_eq!(parse_normalize_header(&["TRUE"]), Ok(Some(true)));
    assert_eq!(parse_normalize_header(&["True"]), Ok(Some(true)));
    assert_eq!(parse_normalize_header(&["false"]), Ok(Some(false)));
    assert_eq!(parse_normalize_header(&["FALSE"]), Ok(Some(false)));
    assert_eq!(parse_normalize_header(&["fAlSe"]), Ok(Some(false)));
}

#[test]
fn normalize_header_invalid_values_rejected() {
    for invalid in ["maybe", "1", "0", "on", "yes", " true", "true ", ""] {
        assert!(
            parse_normalize_header(&[invalid]).is_err(),
            "value {invalid:?} must be rejected"
        );
    }
}

#[test]
fn normalize_header_duplicate_values_rejected() {
    assert!(parse_normalize_header(&["true", "false"]).is_err());
    assert!(parse_normalize_header(&["true", "true"]).is_err());
}

// `X-Normalize` request header contract (spec 009, contracts/openai_embeddings.md).

/// POST a JSON body to `/v1/embeddings` with extra headers; returns status and parsed body.
async fn post_embeddings(
    app: axum::Router,
    body: Value,
    headers: &[(&str, &str)],
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/v1/embeddings")
        .header("Content-Type", "application/json");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let response = app
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn embeddings_without_normalize_header_is_unchanged() {
    let app = test_app(None).await;
    let model_id = fixture_model_id(app.clone()).await;

    let (status, value) =
        post_embeddings(app, json!({"input": ["hello"], "model": model_id}), &[]).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["object"], "list");
    assert!(value["data"][0]["embedding"].is_array());
    assert!(value["usage"]["total_tokens"].is_u64());
}

#[tokio::test]
async fn embeddings_with_normalize_true_and_false_return_embeddings() {
    let app = test_app(None).await;
    let model_id = fixture_model_id(app.clone()).await;

    for header_value in ["true", "false"] {
        let (status, value) = post_embeddings(
            app.clone(),
            json!({"input": ["hello"], "model": model_id}),
            &[("X-Normalize", header_value)],
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(value["object"], "list");
        assert!(value["data"][0]["embedding"].is_array());
    }
}

#[tokio::test]
async fn embeddings_with_normalize_header_and_base64_succeed() {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let app = test_app(None).await;
    let model_id = fixture_model_id(app.clone()).await;

    let (status, value) = post_embeddings(
        app,
        json!({"input": ["hello"], "model": model_id, "encoding_format": "base64"}),
        &[("X-Normalize", "true")],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let encoded = value["data"][0]["embedding"]
        .as_str()
        .expect("base64 embedding should be a string");
    let decoded = STANDARD
        .decode(encoded)
        .expect("base64 payload should decode");
    assert_eq!(decoded.len() % 4, 0, "payload must be whole f32 values");
}

// `X-Normalize` error handling (spec 009 US2).

#[tokio::test]
async fn embeddings_reject_invalid_normalize_value() {
    let app = test_app(None).await;

    for invalid in ["maybe", "1", "on", "yes"] {
        let (status, value) = post_embeddings(
            app.clone(),
            json!({"input": ["hello"]}),
            &[("X-Normalize", invalid)],
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "value {invalid:?}");
        assert_eq!(value["error"], "invalid_request");
        let message = value["message"].as_str().unwrap();
        assert!(message.contains("X-Normalize"), "message: {message}");
        assert!(message.contains("true"), "message: {message}");
    }
}

#[tokio::test]
async fn embeddings_reject_empty_normalize_value() {
    let app = test_app(None).await;

    let (status, value) =
        post_embeddings(app, json!({"input": ["hello"]}), &[("X-Normalize", "")]).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(value["error"], "invalid_request");
}

#[tokio::test]
async fn embeddings_reject_duplicate_normalize_header() {
    let app = test_app(None).await;

    let (status, value) = post_embeddings(
        app,
        json!({"input": ["hello"]}),
        &[("X-Normalize", "true"), ("X-Normalize", "false")],
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(value["error"], "invalid_request");
    assert!(value["message"].as_str().unwrap().contains("once"));
}

#[tokio::test]
async fn body_validation_error_takes_precedence_over_header_error() {
    let app = test_app(None).await;

    let (status, value) =
        post_embeddings(app, json!({"input": []}), &[("X-Normalize", "maybe")]).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    let message = value["message"].as_str().unwrap();
    assert!(
        message.contains("input cannot be empty"),
        "body error must win, got: {message}"
    );
}

// OpenAPI documentation of the extension (spec 009 US3).

#[tokio::test]
async fn openapi_documents_normalize_header() {
    let app = test_app(None).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&body).unwrap();
    let params = value["paths"]["/v1/embeddings"]["post"]["parameters"]
        .as_array()
        .expect("embeddings POST should declare parameters");
    assert!(
        params
            .iter()
            .any(|p| p["name"] == "X-Normalize" && p["in"] == "header"),
        "X-Normalize header parameter must be documented, params: {params:?}"
    );
}
