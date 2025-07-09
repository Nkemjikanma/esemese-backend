use axum::{
    Json, Router, debug_handler, extract,
    extract::Query,
    http::{
        HeaderValue, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    response::IntoResponse,
    routing::{get, post},
};
use dotenv::dotenv;

use http::{Response, header}; // Use http header
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env; // handle env var
use tower_http::cors::{Any, CorsLayer}; // Use http Method // Use http Method

use thiserror::Error;

#[derive(Debug, Error)]
enum ApiError {
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

#[derive(Debug, Serialize, Deserialize)]
struct PinataFile {
    id: String,
    name: String,
    cid: String,
    size: u64,
    number_of_files: u64,
    mime_type: String,
    group_id: String,
    keyvalues: HashMap<String, String>,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PinataFilesData {
    files: Vec<PinataFile>,
    next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PinataFilesResponse {
    data: PinataFilesData,
}

#[derive(Debug, Serialize, Deserialize)]
struct PinataGroup {
    id: String,
    name: String,
    is_public: Option<bool>,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PinataGroupData {
    groups: Vec<PinataGroup>,
    next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PinataGroupResponse {
    data: PinataGroupData,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    groups: Vec<PinataGroup>,
    message: Option<String>,
}

#[tokio::main]
async fn main() {
    // initialize tracking
    tracing_subscriber::fmt::init();

    // .allow_origin(["http://localhost:5173".parse().unwrap(), "https://your-production-domain.com".parse().unwrap()])

    let cors_layer = CorsLayer::new()
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
        .route("/groups", get(get_pinata_groups))
        .route("/groups-with-thumbnails", get(get_groups_with_thumbnails))
        .route("/group-images", get(get_group_images))
        .route("/favourites", get(get_favourites))
        .route("/files-category", get(get_files_by_category))
        .layer(cors_layer);

    // Define Ip and Port
    let address: &'static str = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    // server axum
    axum::serve(listener, app).await.unwrap();
}

// #[debug_handler]
async fn get_pinata_groups() -> Result<Json<ApiResponse>, ApiError> {
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

async fn fetch_groups_from_pinata() -> Result<Vec<PinataGroup>, ApiError> {
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

#[derive(Debug, Deserialize)]
struct GroupImagesParams {
    group_id: Option<String>,
    limit: Option<usize>,
}

#[derive(Serialize)]
struct GroupImagesResponse {
    success: bool,
    group_id: String,
    images: Vec<PinataFile>,
    message: Option<String>,
}

async fn get_group_images(
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

async fn get_favourites(
    query: Query<GroupImagesParams>,
) -> Result<Json<GroupImagesResponse>, ApiError> {
    // Simply delegate to get_group_images
    get_group_images(query).await
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
///////////////// get_files ///////
#[derive(Debug, Deserialize)]
struct CategoryParams {
    categories: Option<String>,
    limit: Option<usize>,
}
#[derive(Serialize)]
struct CategoryResponse {
    success: bool,
    images: Vec<PinataFile>,
    message: Option<String>,
}
async fn get_files_by_category(
    Query(params): Query<CategoryParams>,
) -> Result<Json<CategoryResponse>, ApiError> {
    let categories = match &params.categories {
        Some(cats) => cats
            .split(",")
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>(),
        None => Vec::new(),
    };

    match fetch_files_from_pinata(categories).await {
        Ok(mut files) => {
            // Filter for images only
            // let images: Vec<PinataFile> = files
            //     .into_iter()
            //     .filter(|file| file.mime_type.starts_with("image/"))
            //     .collect();

            // // Apply limit if specified
            if let Some(limit) = params.limit {
                files = files.into_iter().take(limit).collect();
            }

            Ok(Json(CategoryResponse {
                success: true,
                images: files,
                message: None,
            }))
        }
        Err(e) => {
            eprintln!("Error fetching files by categories: {e}");
            Err(e)
        }
    }
}

async fn fetch_files_from_pinata(categories: Vec<String>) -> Result<Vec<PinataFile>, ApiError> {
    dotenv().ok();
    let api_key = env::var("PINATA_JWT").map_err(|e| {
        eprintln!("Failed to get PINATA_JWT: {e}");
        ApiError::Env(e)
    })?;

    let gatewat_key = env::var("PINATA_GATEWAY_KEY").map_err(|e| {
        eprintln!("Failed to get the gatewar key: {e}");
        ApiError::Env(e)
    });

    let client = Client::new();
    let mut all_files = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = Url::parse("https://api.pinata.cloud/v3/files/public")?;

        if !categories.is_empty() {
            let metadata_json = if categories.len() == 1 {
                format!(
                    r#"{{"category":{{"value":"{}","op":"eq"}}}}"#,
                    categories[0]
                )
            } else {
                let categories_json = categories
                    .iter()
                    .map(|c| format!(r#""{}""#, c))
                    .collect::<Vec<_>>()
                    .join(",");

                format!(
                    r#"{{"category":{{"value":[{}],"op":"in"}}}}"#,
                    categories_json
                )
            };

            // // url encode the json
            // let encoded_metadata =
            //
            url.query_pairs_mut()
                .append_pair("metadata[keyvalues]", &metadata_json);

            println!("{url}");
        }

        // add page token
        if let Some(token) = &page_token {
            url.query_pairs_mut().append_pair("pageToken", token);
        }

        println!("{url}");

        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await?;

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

        // parse response
        let data: PinataFilesResponse = response.json().await?;
        println!("Found {} files", data.data.files.len());

        all_files.extend(data.data.files);

        match data.data.next_page_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }

    println!("Total files collected: {}", all_files.len());
    Ok(all_files)
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupWithThumbnail {
    id: String,
    name: String,
    is_public: Option<bool>,
    created_at: String,
    thumbnail_image: Option<PinataFile>,
    photo_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupsWithThumbnailResponse {
    success: bool,
    collections: Vec<GroupWithThumbnail>,
    message: Option<String>,
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
