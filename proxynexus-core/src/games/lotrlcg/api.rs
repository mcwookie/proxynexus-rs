use crate::error::{ProxyNexusError, Result};
use crate::games::fetch_json;
use crate::games::lotrlcg::models::{RingsdbCard, RingsdbDecklist};
use crate::models::{Decklist, DecklistEntry};

const BASE_URL: &str = "https://ringsdb.com/api/public";

pub async fn fetch_all_cards() -> Result<Vec<RingsdbCard>> {
    fetch_json(&format!("{}/cards/", BASE_URL)).await
}

pub async fn fetch_decklist_from_ringsdb(url: &str) -> Result<Decklist> {
    let deck_id = parse_ringsdb_url(url)?;
    let api_url = format!("{}/decklist/{}", BASE_URL, deck_id);
    let decklist_response: RingsdbDecklist = fetch_json(&api_url).await?;

    let all_cards = fetch_all_cards().await?;
    let mut code_to_card = std::collections::HashMap::new();
    for card in all_cards {
        code_to_card.insert(card.code.clone(), card);
    }

    let mut cards = Vec::new();
    for (code, quantity) in decklist_response.slots {
        if let Some(card) = code_to_card.get(&code) {
            let card_id = crate::card_store::normalize_title(&card.name);

            let clean_pack_name = card
                .pack_name
                .replace("ALeP - ", "")
                .replace(".English", "");
            let pack_id = crate::card_store::normalize_title(&clean_pack_name);

            cards.push(DecklistEntry {
                card_id,
                pack_id: Some(pack_id),
                quantity: quantity as u32,
            });
        }
    }

    Ok(Decklist { cards })
}

fn parse_ringsdb_url(url: &str) -> Result<String> {
    extract_path_segment(url, "/decklist/view/")
        .or_else(|| extract_path_segment(url, "/api/public/decklist/"))
        .ok_or_else(|| ProxyNexusError::Internal("Invalid RingsDB decklist URL".into()))
}

fn extract_path_segment(url: &str, segment: &str) -> Option<String> {
    url.split(segment)
        .nth(1)
        .map(|s| {
            s.trim_end_matches('/')
                .split(['/', '?', '#'])
                .next()
                .unwrap_or("")
                .to_string()
        })
        .filter(|s| !s.is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_ringsdb_catalog() -> Result<Vec<RingsdbCard>> {
    let cards_response: Vec<RingsdbCard> =
        fetch_json("https://ringsdb.com/api/public/cards/").await?;
    Ok(cards_response)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_ringsdb_packs() -> Result<Vec<crate::games::lotrlcg::models::RingsdbPack>> {
    let packs_response: Vec<crate::games::lotrlcg::models::RingsdbPack> =
        fetch_json("https://ringsdb.com/api/public/packs/").await?;
    Ok(packs_response)
}
