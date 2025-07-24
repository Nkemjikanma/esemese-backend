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

use crate::models::{
    favourites::{ApiResponse, PinataFilesResponse},
    groups::{GroupWithThumbnail, GroupsWithThumbnailResponse, PinataGroupResponse},
    pinata::{PinataFile, PinataGroup},
};

pub fn groups_router() -> Router {
    Router::new()
        .route("/groups", get(get_pinata_groups))
        .route("/groups-with-thumbnails", get(get_groups_with_thumbnails))
}

pub async fn get_pinata_groups() -> Result<Json<ApiResponse>, ApiError> {
    match fetch_groups_from_pinata().await {
        Ok(groups) => {
            println!("Fetched {} groups", groups.len());

            // Return successful response
            Ok(Json(ApiResponse {
                success: true,
                groups,
                message: None,
            }))
        }
        Err(e) => {
            // Log the error
            eprintln!("Error fetching groups: {e}");

            // Return error response
            Err(e)
        }
    }
}

pub async fn fetch_groups_from_pinata() -> Result<Vec<PinataGroup>, ApiError> {
    dotenv().ok();
    let api_key = env::var("PINATA_JWT").map_err(|e| {
        eprintln!("Failed to get PINATA_JWT: {e}");
        ApiError::Env(e)
    })?;

    let client = Client::new();
    let mut all_groups = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url: String = "https://api.pinata.cloud/v3/groups/public".to_owned();

        // add the page_token as query param if avail
        if let Some(token) = &page_token {
            url = format!("{url}?pageToken={token}");
        }

        // print url
        println!("Requesting URL: {url}");

        // make request
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await?;

        println!("{response:?}");

        // check if successful
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

        // Parse the response
        let data: PinataGroupResponse = response.json().await?;
        println!("Raw API response: {data:?}");

        // add groups to our collection
        all_groups.extend(data.data.groups);

        // check if more to fetch
        match data.data.next_page_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }

    Ok(all_groups)
}

async fn fetch_images_from_group(
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

#[axum::debug_handler]
async fn get_groups_with_thumbnails() -> Result<Json<GroupsWithThumbnailResponse>, ApiError> {
    match fetch_groups_from_pinata().await {
        Ok(groups) => {
            let mut collections = Vec::new();

            for group in groups {
                let result = fetch_images_from_group(&group.id, Some(1)).await;

                let (thumbnail, count) = match result {
                    Ok(files) => {
                        let count = files.len();
                        let thumbnail = files.into_iter().next();
                        (thumbnail, count)
                    }
                    Err(_) => (None, 0),
                };

                collections.push(GroupWithThumbnail {
                    id: group.id,
                    name: group.name,
                    is_public: group.is_public,
                    created_at: group.created_at,
                    thumbnail_image: thumbnail,
                    photo_count: count,
                });
            }

            Ok(Json(GroupsWithThumbnailResponse {
                success: true,
                collections,
                message: None,
            }))
        }
        Err(e) => {
            eprintln!("Error fetching groups with thumbnails: {e}");
            Err(e)
        }
    }
}
