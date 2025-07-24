use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct GroupInfo {
    pub create_new_group: bool,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PhotoUpload {
    pub file: Vec<u8>,
    pub filename: String,
    pub content_type: String,
    pub metadata: PhotoMetadata,
}

#[derive(Debug, Deserialize)]
pub struct PhotoMetadata {
    pub title: String,
    pub description: String,
    pub category: String,
    pub camera: String,
    pub lens: String,
    pub iso: String,
    pub aperture: String,
    #[serde(rename = "shutterSpeed")]
    pub shutter_speed: String, // Remeber - "shutterSpeed" in the JSON
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub success: bool,
    pub files: Vec<UploadedFileInfo>,
    pub group_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UploadedFileInfo {
    pub id: String,
    pub name: String,
    pub cid: String,
    pub group_id: Option<String>, // Other fields returned from Pinata
}

#[derive(Debug, Deserialize)]
pub struct PinataUploadResponse {
    pub data: PinataUploadData,
}

#[derive(Debug, Deserialize)]
pub struct PinataUploadData {
    pub id: String,
    pub name: String,
    pub cid: String,
    pub created_at: String,
    pub size: u64,
    pub number_of_files: u32,
    pub mime_type: String,
    pub group_id: Option<String>,
    pub keyvalues: Option<HashMap<String, String>>,
}
