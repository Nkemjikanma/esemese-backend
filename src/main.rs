use axum::debug_handler;
use dotenv::dotenv;
// use postgres::Error as PostgresError; // Errors
// use postgres::{Client, NoTls}; // for none secure connection
use reqwest::Client;
use std::env; // handle env var
use std::io::{Read, Write}; // to read and write from a tcp stream
use std::net::{TcpListener, TcpStream};
use std::os::macos;
use tokio::time::error;

use axum::{
    Json, Router, extract,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Deserialize)]
struct GroupNameSearch {
    group_id: Option<String>, // Optional to allow default "favorites" group
    limit: Option<usize>,     // Optional limit for number of images
}

#[derive(Serialize)]
struct FavouritesResponse {
    success: bool,
    group_id: String,
    images: Vec<PinataFile>,
    message: Option<String>,
}
// // Database URL
// const DB_URL: &str = !env("DATABASE_URL");

// // Response Constants
// const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
// const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
// const INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

#[tokio::main]
async fn main() {
    // initialize tracking
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/groups", get(get_pinata_groups))
        .route("/favourites", get(get_favourites));
    // .route("/groups/:group_id/files", get(get_group_files));
    // .route("/gallery:collectionId", get(getCollection));
    // .route("/gallery", post(creat_gallery));

    // Define Ip and Port
    let address: &'static str = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    // server axum
    axum::serve(listener, app).await.unwrap();
}

// #[debug_handler]
async fn get_pinata_groups() -> Result<Json<ApiResponse>, (StatusCode, String)> {
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
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch groups: {e}"),
            ))
        }
    }
}

async fn fetch_groups_from_pinata() -> Result<Vec<PinataGroup>, Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = env::var("PINATA_JWT").expect("PINATA JWT Not Found");

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

async fn get_favourites() -> Result<Json<FavouritesResponse>, (StatusCode, String)> {
    let favorites_group_id = "876d949f-6532-44af-924c-f164e5ac6b1b".to_string();

    match fetch_images_from_group(&favorites_group_id).await {
        Ok(files) => {
            // Filter for images only
            let images: Vec<PinataFile> = files
                .into_iter()
                .filter(|file| file.mime_type.starts_with("image/"))
                .collect();

            Ok(Json(FavouritesResponse {
                success: true,
                group_id: favorites_group_id,
                images,
                message: None,
            }))
        }
        Err(e) => {
            eprintln!("Error fetching carousel images: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch carousel images: {e}"),
            ))
        }
    }
}

async fn fetch_images_from_group(
    group_id: &str,
) -> Result<Vec<PinataFile>, Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = env::var("PINATA_JWT").expect("PINATA JWT Not Found");

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

        // check for mmore pages
        match data.data.next_page_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }

    println!("Total files collected: {}", all_files.len());
    Ok(all_files)
}
