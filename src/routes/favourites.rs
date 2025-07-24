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

use crate::errors::ApiError;
use http::{Response, header}; // Use http header
use reqwest::{Client, Request, Url};
use serde::{Deserialize, Serialize};
use std::env; // handle env var
use std::{collections::HashMap, time::Duration};
use tower_http::cors::{Any, CorsLayer}; // Use http Method // Use http Method

use crate::models::favourites::{GroupImagesParams, GroupImagesResponse, PinataFilesResponse};
use crate::models::pinata::PinataFile;

pub fn favourites_router() -> Router {
    Router::new()
        .route("/favourites", get(get_favourites))
        .route("/group-images", get(get_group_images))
}

pub async fn get_favourites(
    query: Query<GroupImagesParams>,
) -> Result<Json<GroupImagesResponse>, ApiError> {
    // Simply delegate to get_group_images
    get_group_images(query).await
}

pub async fn get_group_images(
    Query(params): Query<GroupImagesParams>,
) -> Result<Json<GroupImagesResponse>, ApiError> {
    let group_id = params
        .group_id
        .unwrap_or_else(|| "876d949f-6532-44af-924c-f164e5ac6b1b".to_string());

    match fetch_images_from_group(&group_id, params.limit).await {
        Ok(files) => Ok(Json(GroupImagesResponse {
            success: true,
            group_id,
            images: files,
            message: None,
        })),
        Err(e) => {
            eprintln!("Error fetching carousel images: {e}");
            Err(e)
        }
    }
}

pub async fn fetch_images_from_group(
    group_id: &str,
    limit: Option<usize>,
) -> Result<Vec<PinataFile>, ApiError> {
    dotenv().ok();
    let api_key = env::var("PINATA_JWT").map_err(|e| {
        eprintln!("Failed to get PINATA_JWT: {e}");
        ApiError::Env(e)
    })?;

    let client = Client::new();
    let mut all_files = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = format!(
            "https://api.pinata.cloud/v3/files/public?group={}",
            group_id
        );

        // add the page_token as query param if avail
        if let Some(token) = &page_token {
            url = format!("{url}?pageToken={token}");
        }

        println!("Requesting URL: {}", url);

        // request
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await?;

        println!("{response:?}");

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await?;
            println!("API request failed with status: {status}");
            println!("Response body: {error_body}");
            return Err(format!(
                "API request failed with status: {}. Body: {}",
                status, error_body
            )
            .into());
        }

        let data: PinataFilesResponse = response.json().await?;
        println!("Found {} files in group", data.data.files.len());

        // add files to our collection
        all_files.extend(data.data.files);

        if let Some(limit_val) = limit {
            if all_files.len() >= limit_val {
                all_files.truncate(limit_val);
                break;
            }
        }

        // check for mmore pages
        match data.data.next_page_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }

    println!("Total files collected: {}", all_files.len());
    Ok(all_files)
}
