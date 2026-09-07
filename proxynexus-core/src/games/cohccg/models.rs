use serde::Deserialize;

/// A pack (set/expansion) from `coh_packs.json`.
///
/// Example:
/// ```json
/// {
///   "code": "arena",
///   "name": "Arena",
///   "position": 1,
///   "available": "2005-06-01"
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct CohPack {
    pub code: String,
    pub name: String,
    pub position: i64,
    pub available: Option<String>,
}

/// A card from `coh_full.json` -- one bulk file covering both sets. Each
/// `CohCard` maps 1:1 to one physical card with the single shared City of
/// Heroes CCG back.
///
/// Example:
/// ```json
/// {
///   "id": "arena_011",
///   "name": "Head Shot",
///   "pack_code": "arena",
///   "number": 11,
///   "type": "edge",
///   "rarity": "rare",
///   "image": "CCG_A_011_Head_Shot.png"
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct CohCard {
    /// Unique per-game card id, `{set}_{number:03}` (e.g. `arena_011`,
    /// `so_144`). Matches the `{card_id}` portion of the image naming
    /// convention.
    pub id: String,
    /// The card's name.
    pub name: String,
    /// Matches the `{pack_id}` portion of the image naming convention
    /// (`arena` or `secret_origins`).
    pub pack_code: String,
    /// Collector number printed on the card, within its set.
    pub number: i64,
    /// Card type: edge, power, hero, enhancement, mission, sidekick, or
    /// "sig power".
    #[serde(rename = "type")]
    pub card_type: String,
    /// Printed rarity: `common`, `uncommon`, `rare`, or `battle pack`
    /// (starter- / battle-pack exclusive, never sold in booster packs).
    pub rarity: String,
}
