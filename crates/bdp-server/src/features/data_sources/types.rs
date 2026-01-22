use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinComment {
    pub topic: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinFeature {
    pub feature_type: String,
    pub description: Option<String>,
    pub start_pos: Option<i32>,
    pub end_pos: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinCrossReference {
    pub database: String,
    pub database_id: String,
    pub metadata: Option<serde_json::Value>,
}
