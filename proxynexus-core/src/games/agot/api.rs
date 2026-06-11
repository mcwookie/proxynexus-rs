use crate::error::Result;
use crate::games::agot::models::{AgotCard, AgotPack};

const BASE_URL: &str = "https://thronesdb.com/api/public";

pub async fn fetch_all_packs() -> Result<Vec<AgotPack>> {
    let url = format!("{}/packs/", BASE_URL);
    fetch_json_array(&url).await
}

pub async fn fetch_all_cards() -> Result<Vec<AgotCard>> {
    let url = format!("{}/cards/", BASE_URL);
    fetch_json_array(&url).await
}

async fn fetch_json_array<T: for<'de> serde::Deserialize<'de>>(url: &str) -> Result<Vec<T>> {
    #[cfg(not(target_arch = "wasm32"))]
    let json_str = reqwest::get(url).await?.text().await?;

    #[cfg(target_arch = "wasm32")]
    let json_str = gloo_net::http::Request::get(url)
        .send()
        .await?
        .text()
        .await?;

    let data: Vec<T> = serde_json::from_str(&json_str)?;
    Ok(data)
}
