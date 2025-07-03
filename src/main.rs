use dotenv::dotenv;
use postgres::Error as PostgresError; // Errors
// use postgres::{Client, NoTls}; // for none secure connection
use reqwest::Client;
use std::env; // handle env var
use std::io::{Read, Write}; // to read and write from a tcp stream
use std::net::{TcpListener, TcpStream};
use std::os::macos;
use tokio::time::error;

use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct PinataGroup {
    id: String,
    name: String,
    is_public: bool,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PinataGroupData {
    groups: Vec<PinataGroup>,
    next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PinatatGroupResponse {
    data: PinataGroupData,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    groups: Vec<PinataGroup>,
    message: Option<String>,
}

// // User model with optional ID because that is created in DB
// #[derive(Serialize, Deserialize)]
// struct User {
//     id: Option<i32>,
//     name: String,
//     email: String,
// }

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

    let app = Router::new().route("/", get(root));
    // .route("/gallery:collectionId", get(getCollection));
    // .route("/gallery", post(creat_gallery));

    // Define Ip and Port
    let address: &'static str = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    // server axum
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> Result<Json<ApiResponse>, (StatusCode, String)> {
    match fetch_groups_from_pinata().await {
        Ok(groups) => {
            // Return successful response
            Ok(Json(ApiResponse {
                success: true,
                groups,
                message: None,
            }))
        }
        Err(e) => {
            // Log the error
            eprintln!("Error fetching groups: {}", e);

            // Return error response
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch groups: {}", e),
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
        let mut url: String = format!("https://api.pinata.cloud/v3/groups/public");

        // add the page_token as query param if avail
        if let Some(token) = &page_token {
            url = format!("{}?pageToken={}", url, token);
        }

        // make request
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await?;

        // check if successful
        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()).into());
        }

        // Parse the response
        let data: PinatatGroupResponse = response.json().await?;

        eprintln!("{data:?}");

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
