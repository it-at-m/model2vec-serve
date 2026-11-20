#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

mod common;

// Integration tests for per-model maximum input length resolution (spec 008).

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::test_config_per_model;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const LONG_TEXT: &str = "embedding truncation boundary test. ";

/// POST one input string to a per-model embed endpoint and return the first
/// embedding vector as `f64` values.
async fn embed_text(app: &axum::Router, path_id: &str, long_text: String) -> Vec<f64> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/tei/{path_id}/embed"))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "inputs": long_text }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&body).unwrap();
    value[0]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

#[test]
fn fixture_declares_max_length_to_the_crate() {
    let fixture = common::model_dir_with_max_length(256);
    let model =
        model2vec_rs::model::StaticModel::from_pretrained(&fixture, None, None, None).unwrap();
    assert_eq!(model.max_length(), Some(256));
}

#[tokio::test]
async fn per_model_max_input_length_resolves_from_config() {
    let fixture = common::model_dir_with_max_length(256);
    let reference = common::alt_model_dir();
    let config = test_config_per_model(
        vec![fixture.clone(), reference.clone()],
        Some(fixture.clone()),
        None,
    );
    let state = model2vec_serve::state::AppState::new(config, common::metrics_handle()).unwrap();
    let app = model2vec_serve::routes::app(state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/tei/model-256/info")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["max_input_length"], 256);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/tei/alt-model/info")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["max_input_length"], 512);
}

#[tokio::test]
async fn per_model_truncation_differences_are_visible() {
    let fixture = common::model_dir_with_max_length(256);
    let reference = common::alt_model_dir();
    let config = test_config_per_model(
        vec![fixture.clone(), reference.clone()],
        Some(fixture.clone()),
        None,
    );
    let state = model2vec_serve::state::AppState::new(config, common::metrics_handle()).unwrap();
    let app = model2vec_serve::routes::app(state);

    let long_text = LONG_TEXT.repeat(40);
    let fixture_embedding = embed_text(&app, "model-256", long_text.clone()).await;
    let reference_embedding = embed_text(&app, "alt-model", long_text).await;

    // The same text truncates at 256 tokens for the fixture model and at 512
    // for the reference copy, so the embeddings must differ.
    let max_delta = fixture_embedding
        .iter()
        .zip(reference_embedding.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max);
    assert!(
        max_delta > 1e-4,
        "embeddings should differ when truncation limits differ (max delta {max_delta})"
    );
}

#[tokio::test]
async fn unset_max_input_length_keeps_crate_default() {
    let fixture = common::model_dir_with_max_length(256);
    let reference = common::alt_model_dir();

    // Explicit `MAX_INPUT_LENGTH` still overrides per-model values.
    let mut config = test_config_per_model(
        vec![fixture.clone(), reference.clone()],
        Some(fixture.clone()),
        None,
    );
    config.max_input_length = Some(128);
    let state = model2vec_serve::state::AppState::new(config, common::metrics_handle()).unwrap();
    let app = model2vec_serve::routes::app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/tei/model-256/info")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["max_input_length"], 128);
}
