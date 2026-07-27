use serde::Deserialize;

/// A pack (set/expansion) from MarvelCDB's `/api/public/packs/` endpoint.
///
/// Example:
/// ```json
/// {
///   "code": "core",
///   "name": "Core Set",
///   "position": 1,
///   "date_release": "2019-11-01",
///   "size": 355
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct McdbPack {
    pub code: String,
    pub name: String,
    pub position: i64,
    /// MarvelCDB's release-date field is named "available", not
    /// "date_release" -- confirmed via the raw API response (e.g. Core
    /// Set: `"available": "2019-11-01"`, no "date_release" key at all).
    /// A field literally named `date_release` here would silently
    /// deserialize to `None` for every pack, since serde matches JSON
    /// keys by field name and that key doesn't exist -- which is exactly
    /// what happened before this was renamed, making the Set dropdown's
    /// date-based sort a no-op for every Marvel Champions pack.
    pub available: Option<String>,
    pub size: Option<i64>,
}

/// A card from MarvelCDB's `/api/public/cards/` (or `/api/public/cards/{pack_code}`)
/// endpoint. Covers player cards, heroes/alter-egos, villains, and encounter cards —
/// they all share this shape, just with different fields populated.
///
/// Player card example:
/// ```json
/// {
///   "code": "01001a",
///   "name": "Spider-Man",
///   "pack_code": "core",
///   "position": 1,
///   "type_code": "hero",
///   "faction_code": "hero",
///   "back_link": "01001b",
///   "is_unique": true,
///   "quantity": 1
/// }
/// ```
///
/// Encounter card example:
/// ```json
/// {
///   "code": "01094",
///   "name": "Rhino",
///   "pack_code": "core",
///   "position": 94,
///   "type_code": "villain",
///   "faction_code": "encounter",
///   "set_code": "rhino",
///   "set_position": 1,
///   "stage": "I",
///   "quantity": 1
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct McdbCard {
    /// The card's unique code — matches the `{card_id}` portion of the
    /// image naming convention exactly, no normalization needed.
    pub code: String,
    pub name: String,
    /// Matches the `{pack_id}` portion of the image naming convention.
    pub pack_code: String,
    pub position: i64,
    /// e.g. "hero", "alter_ego", "villain", "minion", "main_scheme",
    /// "side_scheme", "ally", "event", "upgrade", "support", "attachment",
    /// "resource", "treachery", "obligation", "environment".
    pub type_code: String,
    /// Primary player/encounter split signal: "hero" or "encounter" for
    /// most cards, plus the five player-side factions (aggression, justice,
    /// leadership, protection, pool) and "basic"/"campaign".
    pub faction_code: String,
    /// Present on double-sided cards (e.g. hero/alter-ego pairs). Points to
    /// the `code` of this card's other side.
    pub back_link: Option<String>,
    /// True for the "B" side of a double-sided card (e.g. the alter-ego
    /// side) — MarvelCDB lists both sides as separate card entries, with
    /// the back side hidden from normal browsing/search.
    pub hidden: Option<bool>,
    /// Groups a villain's multi-stage forms together (e.g. Rhino I/II/III
    /// all share `set_code: "rhino"`).
    pub set_code: Option<String>,
    pub set_position: Option<i64>,
    /// Villain stage, e.g. "I", "II", "III".
    pub stage: Option<String>,
    pub is_unique: Option<bool>,
    pub quantity: Option<i64>,
}

/// A decklist from MarvelCDB's `/api/public/decklist/{decklist_id}` endpoint.
/// Mirrors the RingsDB shape used by the LotR LCG adapter — card codes
/// mapped to quantities. Kept here for a future `DecklistProvider` impl.
#[derive(Debug, Clone, Deserialize)]
pub struct McdbDecklist {
    pub slots: std::collections::HashMap<String, i64>,
}
