use serde::{Deserialize, Serialize};

use crate::PinataFile;

#[derive(Debug, Deserialize)]
pub struct CategoryParams {
    pub categories: Option<String>,
    pub limit: Option<usize>,
}
#[derive(Serialize)]
pub struct CategoryResponse {
    pub success: bool,
    pub images: Vec<PinataFile>,
    pub message: Option<String>,
}
