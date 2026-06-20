use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AgotCard {
    pub code: String,
    pub name: String,
    pub label: String,
    pub pack_code: String,
    pub type_code: String,
    pub quantity: i64,
    pub position: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgotPack {
    pub code: String,
    pub name: String,
    pub available: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgotDecklist {
    pub slots: std::collections::HashMap<String, u32>,
}
