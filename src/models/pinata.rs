use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct PinataFile {
    pub id: String,
    pub name: String,
    pub cid: String,
    pub size: u64,
    pub number_of_files: u64,
    pub mime_type: String,
    pub group_id: String,
    pub keyvalues: HashMap<String, String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PinataGroup {
    pub id: String,
    pub name: String,
    pub is_public: Option<bool>,
    pub created_at: String,
}
