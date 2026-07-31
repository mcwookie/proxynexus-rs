use serde::Deserialize;

/// A pack (set/expansion) from ArkhamDB's `/api/public/packs/` endpoint.
///
/// Example:
/// ```json
/// {
///   "code": "core",
///   "name": "Core Set",
///   "position": 1,
///   "available": "2016-11-10",
///   "known": 185,
///   "total": 184
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct AhdbPack {
    pub code: String,
    pub name: String,
    pub position: i64,
    /// ArkhamDB's release-date field is named "available", not "date_release".
    pub available: Option<String>,
}

/// A card from ArkhamDB's `/api/public/cards/{pack_code}` endpoint. Covers
/// investigator, player, and encounter cards -- they all share this shape,
/// just with different fields populated.
///
/// Player card example:
/// ```json
/// {
///   "code": "01001",
///   "name": "Roland Banks",
///   "pack_code": "core",
///   "position": 1,
///   "type_code": "investigator",
///   "faction_code": "guardian",
///   "double_sided": true,
///   "quantity": 1
/// }
/// ```
///
/// Encounter card example:
/// ```json
/// {
///   "code": "01104",
///   "name": "The Gathering",
///   "pack_code": "core",
///   "position": 104,
///   "type_code": "scenario",
///   "faction_code": "mythos",
///   "quantity": 1
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct AhdbCard {
    /// The card's unique code -- matches the `{card_id}` portion of the
    /// image naming convention exactly, no normalization needed.
    pub code: String,
    pub name: String,
    /// Matches the `{pack_id}` portion of the image naming convention.
    pub pack_code: String,
    pub position: i64,
    /// e.g. "investigator", "asset", "event", "skill", "enemy", "treachery",
    /// "act", "agenda", "location", "scenario", "story". Used by the
    /// adapter's `back_type_for` to classify which generic card back
    /// (player/encounter) a card needs -- see that function's doc comment
    /// for why `faction_code` alone isn't reliable for this.
    pub type_code: String,
    /// Player faction ("guardian", "seeker", "rogue", "mystic", "survivor",
    /// "neutral") or "mythos" for encounter cards.
    pub faction_code: String,
    #[serde(default)]
    pub quantity: Option<i64>,
    /// True for cards with a printed back side (most investigators, acts,
    /// and agendas). Unlike MarvelCDB, ArkhamDB keeps both sides under this
    /// single `code` -- `imagesrc`/`backimagesrc` point to the front/back
    /// art rather than issuing the back its own card entry. That maps
    /// directly onto Proxy Nexus's `{card_id}@{pack_id}~back` part-naming
    /// convention, so no card-splitting is needed here (contrast with the
    /// MarvelCDB adapter's linked_card flattening).
    #[allow(dead_code)]
    #[serde(default)]
    pub double_sided: Option<bool>,
}

/// A decklist from ArkhamDB's `/api/public/decklist/{id}` endpoint. Mirrors
/// the MarvelCDB/RingsDB shape -- card codes mapped to quantities. Does not
/// include the investigator itself (that's a separate `investigator_code`
/// field), matching how every other adapter's decklist parsing leaves the
/// identity/investigator card for the user to add separately.
#[derive(Debug, Clone, Deserialize)]
pub struct AhdbDecklist {
    pub slots: std::collections::HashMap<String, i64>,
}
