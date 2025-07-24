use axum::{Json, extract::Query};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{PinataFile, PinataGroup};
use crate::ApiError;

#[derive(Debug, Serialize, Deserialize)]
pub struct PinataFilesData {
    pub files: Vec<PinataFile>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PinataFilesResponse {
    pub data: PinataFilesData,
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub groups: Vec<PinataGroup>,
    pub message: Option<String>,
}

// #[debug_handler]
#[derive(Debug, Deserialize)]
pub struct GroupImagesParams {
    pub group_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct GroupImagesResponse {
    pub success: bool,
    pub group_id: String,
    pub images: Vec<PinataFile>,
    pub message: Option<String>,
}
