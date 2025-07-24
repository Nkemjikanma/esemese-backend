use axum::{Json, Router, extract::multipart::Multipart, routing::post};
use reqwest::Client;

use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use std::time::Duration;

use crate::errors::ApiError;
use crate::models::{
    groups::GroupCreationResponse,
    uploads::{PhotoMetadata, PinataUploadResponse, UploadResponse, UploadedFileInfo},
};

pub fn uploads_router() -> Router {
    Router::new().route("/upload", post(upload_photo))
}

pub async fn upload_photo(mut multipart: Multipart) -> Result<Json<UploadResponse>, ApiError> {
    println!("Processing upload request");

    let mut create_new_group = false;
    let mut group_id: Option<String> = None;
    let mut group_name: Option<String> = None;

    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    let mut file_names: HashMap<String, String> = HashMap::new();
    let mut metadata_map: HashMap<String, PhotoMetadata> = HashMap::new();

    while let Some(field) = match multipart.next_field().await {
        Ok(Some(f)) => Some(f),
        Ok(None) => None,
        Err(e) => {
            println!("Error reading next field: {e}",);
            return Err(ApiError::Api(format!(
                "Failed to process multipart form: {e}",
            )));
        }
    } {
        let name = field.name().unwrap_or("").to_string();

        if name == "createNewGroup" {
            let value = field.text().await.map_err(|err| {
                ApiError::Api(format!("Failed to read createNewGroup field: {err}"))
            })?;
            create_new_group = value.parse::<bool>().unwrap_or(false);
        } else if name == "groupId" {
            group_id =
                Some(field.text().await.map_err(|err| {
                    ApiError::Api(format!("Failed to read groupId field: {err}"))
                })?);
        } else if name == "groupName" {
            group_name = Some(field.text().await.map_err(|err| {
                ApiError::Api(format!("Failed to read groupName field: {}", err))
            })?);
        } else if name.starts_with("file_") {
            // This is the field for the file
            let file_id = name.clone();
            let file_name = field.file_name().unwrap_or("unnamed_file").to_string();

            match field.bytes().await {
                Ok(data) => {
                    println!("File data size: {} bytes", data.len());
                    files.insert(file_id.clone(), data.to_vec());
                    file_names.insert(file_id, file_name);
                }
                Err(e) => {
                    println!("Failed to read file data: {}", e);
                    return Err(ApiError::Api(format!("Failed to read file data: {}", e)));
                }
            }
        } else if name.starts_with("metadata_") {
            // extract the file's unique id from metadata_{file_id}
            let fie_id = name.strip_prefix("metadata_").unwrap_or("").to_string();
            let metadata_str = field
                .text()
                .await
                .map_err(|e| ApiError::Api(format!("Failed to read metadata: {}", e)))?;

            let metadata: PhotoMetadata = match serde_json::from_str(&metadata_str) {
                Ok(m) => m,
                Err(err) => {
                    println!("Failed to parse metadata JSON: {err}",);
                    return Err(ApiError::Api(format!(
                        "Failed to parse metadata JSON: {err}",
                    )));
                }
            };

            metadata_map.insert(fie_id, metadata);
        }
    }

    // upload each file to pinata
    let mut uploaded_files = Vec::new();
    let mut created_group_id: Option<String> = None;

    for (file_id, file_data) in files {
        let metadata = metadata_map
            .get(&file_id)
            .ok_or_else(|| ApiError::Api(format!("Missing metadata for file: {}", file_id)))?;

        let filename = file_names.get(&file_id).unwrap_or(&file_id).clone();

        // upload functionality eg
        let pinata_result = upload_to_pinata(
            &file_data,
            &filename,
            metadata,
            create_new_group,
            &group_id,
            &group_name,
        )
        .await?;

        // if this is the first file and we created group, store the group ID
        if create_new_group && created_group_id.is_none() {
            created_group_id = pinata_result.group_id.clone();
        }

        uploaded_files.push(pinata_result);
    }

    //
    let response_group_id = if create_new_group {
        created_group_id
    } else {
        group_id
    };

    Ok(Json(UploadResponse {
        success: true,
        files: uploaded_files,
        group_id: response_group_id,
        message: None,
    }))
}

async fn send_pinata_request(
    client: &Client,
    api_key: &str,
    form: reqwest::multipart::Form,
) -> Result<UploadedFileInfo, ApiError> {
    let response = client
        .post("https://uploads.pinata.cloud/v3/files")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| ApiError::Request(e))?;

    // check if successful
    let status = response.status();

    if !status.is_success() {
        let error_body = response.text().await?;
        return Err(ApiError::Api(format!(
            "Pinata API error ({}): {}",
            status, error_body
        )));
    }

    // parse the response to JSON
    let data: PinataUploadResponse = response.json().await?;
    println!("Raw API response: {data:?}");

    let file_info = UploadedFileInfo {
        id: data.data.id,
        name: data.data.name,
        cid: data.data.cid,
        group_id: data.data.group_id,
    };

    Ok(file_info)
}

async fn upload_to_pinata(
    file_data: &[u8],
    filename: &String,
    metadata: &PhotoMetadata,
    create_new_group: bool,
    group_id: &Option<String>,
    group_name: &Option<String>,
) -> Result<UploadedFileInfo, ApiError> {
    dotenv().ok();
    let api_key = env::var("PINATA_JWT").map_err(|e| {
        eprintln!("Failed to get PINATA_JWT: {e}");
        ApiError::Env(e)
    })?;

    // creat client, with retry abilities
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| ApiError::Request(e))?;

    let mut retries = 0;
    let max_retries = 3;
    let mut last_error = None;

    // group creation
    let created_group_id = if create_new_group {
        if let Some(name) = group_name {
            // create the group and get_id
            match create_pinata_group(&client, &api_key, name).await {
                Ok(id) => {
                    println!("Created new group with ID: {}", id);
                    Some(id)
                }
                Err(e) => {
                    println!("Failed to create group: {:?}", e);
                    return Err(e);
                }
            }
        } else {
            return Err(ApiError::Api(
                "Group name is needed for new group creations".to_string(),
            ));
        }
    } else {
        group_id.clone()
    };

    // On each retry, recreate multipart form for Pinata inside a closure
    let create_form = || -> Result<reqwest::multipart::Form, ApiError> {
        let mut form = reqwest::multipart::Form::new()
            .text("network", "public")
            .part(
                "file",
                reqwest::multipart::Part::bytes(file_data.to_vec())
                    .file_name(filename.to_string())
                    .mime_str("multipart/form-data")
                    .map_err(|e| ApiError::Api(format!("Invalid MIME type: {}", e)))?,
            )
            .text("name", metadata.title.clone());

        if let Some(gid) = &created_group_id {
            form = form.text("group_id", gid.clone());
        }

        // convert metadata into Pinata flat format
        let mut keyvalues = HashMap::new();
        keyvalues.insert("category".to_string(), metadata.category.clone());

        if !metadata.description.is_empty() {
            keyvalues.insert("description".to_string(), metadata.description.clone());
        }

        if !metadata.camera.is_empty() {
            keyvalues.insert("camera".to_string(), metadata.camera.clone());
        }

        if !metadata.lens.is_empty() {
            keyvalues.insert("lens".to_string(), metadata.lens.clone());
        }

        if !metadata.iso.is_empty() {
            keyvalues.insert("iso".to_string(), metadata.iso.clone());
        }

        if !metadata.aperture.is_empty() {
            keyvalues.insert("aperture".to_string(), metadata.aperture.clone());
        }

        if !metadata.shutter_speed.is_empty() {
            keyvalues.insert("shutterSpeed".to_string(), metadata.shutter_speed.clone());
        }

        // add keyvalues to JSON
        let keyvalues_json = serde_json::to_string(&keyvalues).map_err(|e| ApiError::Json(e))?;
        let form = form.text("keyvalues", keyvalues_json);

        Ok(form)
    };

    while retries < max_retries {
        // Create a new form for each attempt
        let form = create_form()?;

        match send_pinata_request(&client, &api_key, form).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                // Only retry on certain error types
                match &e {
                    ApiError::Request(req_err) if req_err.is_timeout() || req_err.is_connect() => {
                        // Network error, retry
                        retries += 1;
                        let delay = 2u64.pow(retries as u32) * 1000; // Exponential backoff
                        eprintln!(
                            "Retrying Pinata upload after {}ms (attempt {}/{})",
                            delay, retries, max_retries
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        last_error = Some(e);
                    }
                    _ => return Err(e), // Non-retryable error
                }
            }
        }
    }

    // If we got here, all retries failed
    Err(last_error.unwrap_or_else(|| ApiError::Api("Maximum retries exceeded".to_string())))
}

async fn create_pinata_group(
    client: &Client,
    api_key: &str,
    group_name: &str,
) -> Result<String, ApiError> {
    println!("Creating new Pinata group: {}", group_name);

    // group creation payload
    let group_payload = serde_json::json!({
        "name": group_name,
        "is_public": true
    });

    let response = client
        .post("https://api.pinata.cloud/groups")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&group_payload)
        .send()
        .await
        .map_err(|e| ApiError::Request(e))?;

    let status = response.status();

    if !status.is_success() {
        let error_body = response.text().await?;
        return Err(ApiError::Api(format!(
            "Pinata API error ({}): {}",
            status, error_body
        )));
    }

    let data: GroupCreationResponse = response.json().await.map_err(|e| ApiError::Request(e))?;
    println!("Group creation response: {:?}", data);

    Ok(data.id)
}
