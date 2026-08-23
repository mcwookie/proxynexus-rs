use crate::error::Result;
use crate::games::whinvasion::models::{WhiCard, WhiPack};

/// Fetches all packs (sets/expansions). All data is stored in a single JSON
/// file, so this is a single request.
pub async fn fetch_packs() -> Result<Vec<WhiPack>> {
    // Open whi_packs.json file. This is stored locally
    //  in the repository, so we can just read it directly.
    Ok(serde_json::from_str(include_str!("whi_packs.json"))?)
}

/// Fetches every card across every pack.
/// `whi_full.json` is one bulk file covering
/// the whole catalog, so this is also a single request.
pub async fn fetch_all_cards() -> Result<Vec<WhiCard>> {
    // Open whi_full.json file. This is stored locally
    //  in the repository, so we can just read it directly.
    Ok(serde_json::from_str(include_str!("whi_full.json"))?)
    
}
