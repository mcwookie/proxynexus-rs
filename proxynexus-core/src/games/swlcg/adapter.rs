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

// Which generic card back a card needs, classified by `side`
#[cfg(not(target_arch = "wasm32"))]
const LIGHT_BACK: &str = "light_back";
const DARK_BACK: &str = "dark_back";
const BOTF_LIGHT_BACK: &str = "l01_back";
const BOTF_DARK_BACK: &str = "d01_back";

#[cfg(not(target_arch = "wasm32"))]
fn back_group_for(side: &str, objective_set_number: Option<i64>) -> Option<String> {
    if objective_set_number.is_none() {
        if side == "Light" {
            Some(BOTF_LIGHT_BACK.to_string())
        } else if side == "Dark" {
            Some(BOTF_DARK_BACK.to_string())
        }
    } else {
        if side == "Light" {    
            Some(LIGHT_BACK.to_string())
        } else if side == "Dark" {
            Some(DARK_BACK.to_string())
        }
    }
}
