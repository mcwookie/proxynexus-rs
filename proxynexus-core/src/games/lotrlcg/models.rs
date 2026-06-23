use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct HobCardFront {
    pub image_path: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct HobCard {
    pub title: String,
    pub slug: String,
    pub card_set: String,
    pub number: i64,
    pub quantity: Option<i64>,
    pub front: Option<HobCardFront>,
    pub card_type: String,
}
