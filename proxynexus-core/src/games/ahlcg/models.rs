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
    /// True for cards with a printed back side that's just the flip side of
    /// this same card (most investigators, acts, and agendas) -- ArkhamDB
    /// keeps both sides under this single `code`, with `imagesrc`/
    /// `backimagesrc` pointing to the front/back art rather than issuing
    /// the back its own card entry. That maps directly onto Proxy Nexus's
    /// `{card_id}@{pack_id}~back` part-naming convention, so no
    /// card-splitting is needed for these -- deliberately does NOT affect
    /// `back_type`, which stays based on this card's own `type_code`
    /// regardless (the back is still always the same player/encounter
    /// identity as the front for this category).
    #[allow(dead_code)]
    #[serde(default)]
    pub double_sided: Option<bool>,
    /// Set instead of (or, rarely, alongside) `double_sided` when this
    /// card's back is a MECHANICALLY DIFFERENT card -- e.g. an ally that
    /// flips into an enemy (Carl Sanford, "The Midwinter Gala") -- rather
    /// than just the flip side of the same identity. Confirmed via a live
    /// scan of all 113 ArkhamDB packs: 464 cards carry this, 52 of which
    /// have a front/back that classify to a *different* back_type (player
    /// vs encounter) -- e.g. Carl Sanford himself is `asset`/player up
    /// front but his linked `71034b` is `enemy`/encounter. The linked card
    /// never appears as its own top-level entry in any per-pack listing
    /// (confirmed empirically), only nested here -- see `linked_card`.
    #[serde(default)]
    pub linked_to_code: Option<String>,
    /// The full linked card's data, embedded inline by ArkhamDB's API
    /// (present whenever `linked_to_code` is). Used only for its
    /// `code`/`name`/`type_code` -- surfaced in the manifest as
    /// `linked_card_code`/`linked_card_name`/`linked_card_back_type` so
    /// generation output can show what the physical card's back actually
    /// is, without changing this card's own `back_type`.
    #[serde(default)]
    pub linked_card: Option<Box<AhdbLinkedCard>>,
}

/// The subset of a linked card's fields we actually need -- see
/// `AhdbCard::linked_card`.
#[derive(Debug, Clone, Deserialize)]
pub struct AhdbLinkedCard {
    pub code: String,
    pub name: String,
    pub type_code: String,
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
