use axum::{Json, extract::Query};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{PinataFile, PinataGroup};
use crate::ApiError;

#[derive(Debug, Serialize, Deserialize)]
pub struct PinataGroupData {
    pub groups: Vec<PinataGroup>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PinataGroupResponse {
    pub data: PinataGroupData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupWithThumbnail {
    pub id: String,
    pub name: String,
    pub is_public: Option<bool>,
    pub created_at: String,
    pub thumbnail_image: Option<PinataFile>,
    pub photo_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupsWithThumbnailResponse {
    pub success: bool,
    pub collections: Vec<GroupWithThumbnail>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GroupCreationResponse {
    pub id: String,
    pub user_id: String,
    pub name: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}
