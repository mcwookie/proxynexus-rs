#[cfg(not(target_arch = "wasm32"))]
use crate::card_store::normalize_title;
#[cfg(not(target_arch = "wasm32"))]
use crate::catalog::{Card, CardVersion, Catalog, CatalogProvider, Pack};
#[cfg(not(target_arch = "wasm32"))]
use crate::error::Result;
use crate::games::GameAdapterInfo;
#[cfg(not(target_arch = "wasm32"))]
use crate::games::agot1st::models::{SwlcgCard, SwlcgPack};
#[cfg(not(target_arch = "wasm32"))]
use async_trait::async_trait;

pub struct SwlcgAdapter {}

impl Default for SwlcgAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SwlcgAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl GameAdapterInfo for SwlcgAdapter {
    fn game_id(&self) -> &'static str {
        "swlcg"
    }

    fn game_name(&self) -> &'static str {
        "Star Wars LCG"
    }

    fn subdomains(&self) -> Vec<&'static str> {
        vec!["swlcg"]
    }
}

// Which generic card back a card needs, classified by `side`.
// Balance of the Force packs have different backs than other packs, 
// so we need to check for that as well.
#[cfg(not(target_arch = "wasm32"))]
const LIGHT_BACK: &str = "light";
#[cfg(not(target_arch = "wasm32"))]
const DARK_BACK: &str = "dark";
#[cfg(not(target_arch = "wasm32"))]
const BOTF_LIGHT_BACK: &str = "botf-light";
#[cfg(not(target_arch = "wasm32"))]
const BOTF_DARK_BACK: &str = "botf-dark";

#[cfg(not(target_arch = "wasm32"))]
fn back_group_for(pack_code: &str, side: &str, objective_set_number: Option<i64>) -> Option<String> {
    let is_botf_pack = pack_code == "balance-of-the-force" && objective_set_number.is_some();
    match (is_botf_pack, side) {
        (true, "Light") => Some(BOTF_LIGHT_BACK.to_string()),
        (true, "Dark") => Some(BOTF_DARK_BACK.to_string()),
        (false, "Light") => Some(LIGHT_BACK.to_string()),
        (false, "Dark") => Some(DARK_BACK.to_string()),
        _ => None,
    }
}
