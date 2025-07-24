use thiserror::Error;

use axum::{
    Json, Router, debug_handler,
    extract::{self, DefaultBodyLimit, Query, multipart::Multipart},
    http::{
        HeaderValue, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    response::IntoResponse,
    routing::{get, post},
};
use reqwest;
use serde_json;
use std::env;
use url;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Environment variable error: {0}")]
    Env(#[from] std::env::VarError),

    #[error("HTTP request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("API error: {0}")]
    Api(String),

    #[error("Url parsing error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
}

// function to conver error into axum responses
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        eprintln!("API Error: {self}"); // Log all errors
        let (status, error_message) = match self {
            Self::Env(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Server configuration error",
            ),
            Self::Request(_) => (
                StatusCode::BAD_GATEWAY,
                "Error communicating with external service",
            ),
            Self::UrlParse(_) => (StatusCode::INTERNAL_SERVER_ERROR, "URL parsing error"),
            Self::Api(_) => (StatusCode::BAD_GATEWAY, "External API error"),
            Self::Json(_) => (StatusCode::INTERNAL_SERVER_ERROR, "JSON parsing error"),
        };

        let body = Json(serde_json::json!({
            "success": false,
            "error": error_message,
            "message": self.to_string(),
        }));

        (status, body).into_response()
    }
}

impl From<String> for ApiError {
    fn from(message: String) -> Self {
        ApiError::Api(message)
    }
}
