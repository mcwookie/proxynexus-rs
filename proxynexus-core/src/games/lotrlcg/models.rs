use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct RingsDbPack {
    pub code: Option<String>,
    pub pack_code: Option<String>,
    pub name: String,
    pub date_release: Option<String>,
}

impl RingsDbPack {
    pub fn get_id(&self) -> String {
        self.code.clone().unwrap_or_else(|| self.pack_code.clone().unwrap_or_default())
    }
}

#[derive(Deserialize, Debug)]
pub struct RingsDbCard {
    pub code: String,
    pub pack_code: String,
    pub name: String,
    pub type_code: String,
    pub encounter_name: Option<String>,
    pub position: Option<i64>,
    pub quantity: Option<i64>,
}
