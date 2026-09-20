use serde::Deserialize;

/// A pack (set/expansion) from `meccg_packs.json`.
///
/// Example:
/// ```json
///   {
///     "code": "against-the-shadow",
///     "name": "Against the Shadow",
///     "position": 5,
///     "total_cards": 170,
///     "total_unique": 170,
///     "available": "1997-08"
///   },
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct MeccgPack {
    pub code: String,
    pub name: String,
    pub position: i64,
    pub available: Option<String>,
}

/// A card from `meccg_card_ids.json`.
///
/// Example:
/// ```json
///   {
///     "id": "AS-9",
///     "unique_id": "05009",
///     "card_number": 9,
///     "set": "Against the Shadow",
///     "name": "Dwarven Travelers",
///     "pack_code": "against-the-shadow",
///     "card_type": "Hazard",
///     "alignment": "Neutral",
///     "rarity": "C3",
///     "label": "Dwarven Travelers"
///   },
/// ```

#[derive(Debug, Clone, Deserialize)]
pub struct MeccgCard {
    /// The card's unique code. Matches the `{card_id}` portion of the image
    /// naming convention.
    pub unique_id: String,
    /// The card's name.
    pub name: String,
    /// The card's label. Combination of the card's name and its pack code, to resolve
    /// duplicate names across packs. Example: "Isengard (the-white-hand)".
    pub label: String,
    /// Matches the `{pack_id}` portion of the image naming convention.
    pub pack_code: String,
    /// Card type. Options: Hazard, Resource, Character, Site, Region.
    pub card_type: String,
    /// Card's alignment. Options: Neutral, Hero, Fallen-wizard, Minion,
    /// Balrog, Dual.
    pub alignment: String,
    /// The card's position within its pack. Sent as an integer.
    pub card_number: i64,
    /// Printed rarity: F3, U, C4, R3, etc. (there are many options).
    /// The number indicates the number of copies of this card on a print sheet.
    pub rarity: String,
}
