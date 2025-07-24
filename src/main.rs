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
use dotenv::dotenv;

use http::{Response, header}; // Use http header
use reqwest::{Client, Request, Url};
use serde::{Deserialize, Serialize};
use std::env; // handle env var
use std::{collections::HashMap, time::Duration};
use tower_http::cors::{Any, CorsLayer}; // Use http Method // Use http Method

pub mod errors;
pub mod models;
pub mod routes;
use crate::errors::ApiError;
use crate::models::{favourites::PinataFilesResponse, pinata::PinataFile};
use crate::routes::{
    categories::categories_router, favourites::favourites_router, groups::groups_router,
    uploads::uploads_router,
};

#[tokio::main]
async fn main() {
    // initialize tracking
    tracing_subscriber::fmt::init();

    // .allow_origin(["http://localhost:5173".parse().unwrap(), "https://your-production-domain.com".parse().unwrap()])

    let cors_layer = CorsLayer::new()
        .allow_origin(["http://localhost:5173".parse().unwrap()])
        .allow_methods(Any)
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        // .allow_credentials(true)
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::ORIGIN,
        ]);

    let app = Router::new()
        .merge(groups_router())
        .merge(favourites_router())
        .merge(categories_router())
        .merge(uploads_router())
        .layer(DefaultBodyLimit::disable())
        .layer(cors_layer);

    // Define Ip and Port
    let address: &'static str = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    // server axum
    axum::serve(listener, app).await.unwrap();
}
