use axum::{Json, Router, extract::Query, routing::get};
use dotenv::dotenv;
use reqwest::{Client, Url};
use std::env;

use crate::ApiError;
use crate::models::{
    categories::{CategoryParams, CategoryResponse},
    favourites::PinataFilesResponse,
    pinata::PinataFile,
};

pub fn categories_router() -> Router {
    Router::new().route("/files-category", get(get_files_by_category))
}
pub async fn get_files_by_category(
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

///////////////// get_files ///////
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
