#[cfg(not(target_arch = "wasm32"))]
use crate::card_store::normalize_title;
#[cfg(not(target_arch = "wasm32"))]
use crate::catalog::{Card, CardVersion, Catalog, CatalogProvider, Pack};
#[cfg(not(target_arch = "wasm32"))]
use crate::error::Result;
use crate::games::GameAdapterInfo;
#[cfg(not(target_arch = "wasm32"))]
use crate::games::marvel_champions::api::{fetch_all_cards, fetch_packs};
#[cfg(not(target_arch = "wasm32"))]
use async_trait::async_trait;

pub struct MarvelChampionsAdapter {}

impl Default for MarvelChampionsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MarvelChampionsAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl GameAdapterInfo for MarvelChampionsAdapter {
    fn game_id(&self) -> &'static str {
        "marvel_champions"
    }

    fn game_name(&self) -> &'static str {
        "Marvel Champions"
    }

    fn subdomains(&self) -> Vec<&'static str> {
        vec!["marvel"]
    }
}

// Which generic card back a card needs, classified by `type_code` rather
// than `faction_code`/side. Unlike ArkhamDB, faction_code alone happened
// to agree with type_code everywhere checked here (e.g. every "obligation"
// card -- the "one card in a hero's own pack that uses the encounter
// back" case -- also has faction_code "encounter"). type_code is still
// used, for one general rule shared with the ahlcg adapter rather than a
// different rule per game, and because it correctly distinguishes real
// edge cases faction_code can't: "side_scheme" (encounter) vs
// "player_side_scheme" (player, despite the name) are different
// type_code values entirely, with player_side_scheme's faction_code
// varying per hero (e.g. "leadership", "aggression") rather than being a
// single consistent value to check against. type_code values confirmed
// exhaustively against a full-catalog sample (not just Core Set) as of
// this writing; anything not in either list below is left unclassified
// (`None`) rather than guessed.
//
// "villain" gets its own bucket rather than folding into ENCOUNTER_TYPES:
// the physical Villain card (and its per-stage flip sides) has dedicated
// back art distinct from the rest of the encounter deck, and we have a
// dedicated `marvel_champions_villain_back.png` to match -- unlike the
// ahlcg adapter, which only distinguishes player/encounter.
#[cfg(not(target_arch = "wasm32"))]
const PLAYER_TYPES: &[&str] = &[
    "hero",
    "alter_ego",
    "ally",
    "event",
    "upgrade",
    "support",
    "resource",
    "player_side_scheme",
];
#[cfg(not(target_arch = "wasm32"))]
const VILLAIN_TYPES: &[&str] = &["villain"];
#[cfg(not(target_arch = "wasm32"))]
const ENCOUNTER_TYPES: &[&str] = &[
    "minion",
    "main_scheme",
    "side_scheme",
    "attachment",
    "treachery",
    "obligation",
    "environment",
    "leader",
    "evidence_means",
    "evidence_motive",
    "evidence_opportunity",
];

#[cfg(not(target_arch = "wasm32"))]
fn back_group_for(type_code: &str) -> Option<String> {
    if PLAYER_TYPES.contains(&type_code) {
        Some("player".to_string())
    } else if VILLAIN_TYPES.contains(&type_code) {
        Some("villain".to_string())
    } else if ENCOUNTER_TYPES.contains(&type_code) {
        Some("encounter".to_string())
    } else {
        None
    }
}

/// Turns the flat, per-pack-fetched `McdbCard` list into catalog rows.
///
/// Confirmed live (MarvelCDB API + official rules): a hero's alter-ego
/// side is NOT a separately printable card -- it's the *back* of the same
/// physical identity card players flip between Hero and Alter-Ego, exactly
/// like an ArkhamDB investigator's front/back. MarvelCDB's own
/// `double_sided: false` flag on both sides is misleading here; the real
/// signal is `hidden` (see `McdbCard::hidden`'s doc comment) -- the hidden
/// side must not get its own Card/CardVersion or show up independently in
/// search/generation. Needed a live scan to confirm scope before trusting
/// this: 69 hero cards across the whole catalog carry a `back_link`, all
/// following this shape.
///
/// `card.back_link` has no analogue in the new back_group/CardSide model, so
/// it's carried here as fork-only metadata for `manifest.rs` instead --
/// purely informational, since the back *image* already resolves
/// independently via the `{card_id}@{pack_id}~back` filename convention.
#[cfg(not(target_arch = "wasm32"))]
fn build_cards_and_versions(
    mcdb_cards: &[crate::games::marvel_champions::models::McdbCard],
) -> (Vec<Card>, Vec<CardVersion>) {
    let mut cards = Vec::with_capacity(mcdb_cards.len());
    let mut card_versions = Vec::with_capacity(mcdb_cards.len());

    let by_code: std::collections::HashMap<
        &str,
        &crate::games::marvel_champions::models::McdbCard,
    > = mcdb_cards.iter().map(|c| (c.code.as_str(), c)).collect();

    for card in mcdb_cards {
        if card.hidden == Some(true) {
            continue;
        }

        let linked = card.back_link.as_deref().and_then(|code| by_code.get(code));

        cards.push(Card {
            id: card.code.clone(),
            title: card.name.clone(),
            title_normalized: normalize_title(&card.name),
            back_group: back_group_for(&card.type_code),
            linked_card_code: card.back_link.clone(),
            linked_card_name: linked.map(|l| l.name.clone()),
            linked_card_back_group: linked.and_then(|l| back_group_for(&l.type_code)),
        });

        card_versions.push(CardVersion {
            card_id: card.code.clone(),
            pack_id: card.pack_code.clone(),
            quantity: card.quantity.unwrap_or(1),
            position: Some(card.position),
            api_id: None,
        });
    }

    (cards, card_versions)
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl CatalogProvider for MarvelChampionsAdapter {
    async fn fetch_catalog(&self) -> Result<Catalog> {
        let mcdb_packs = fetch_packs().await?;
        // fetch_all_cards() fetches per-pack rather than the bulk
        // /api/public/cards/ endpoint -- confirmed the bulk endpoint
        // silently drops some encounter cards (see api.rs docs). Slower
        // (~60 requests instead of 1) but the catalog is actually complete.
        let mcdb_cards = fetch_all_cards(&mcdb_packs).await?;

        let packs: Vec<Pack> = mcdb_packs
            .into_iter()
            .map(|pack| Pack {
                id: pack.code,
                name: pack.name,
                date_release: pack.available,
            })
            .collect();

        let (cards, card_versions) = build_cards_and_versions(&mcdb_cards);

        Ok(Catalog {
            game_id: self.game_id().to_string(),
            display_name: self.game_name().to_string(),
            packs,
            cards,
            card_versions,
        })
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::games::marvel_champions::models::McdbCard;

    fn hero(code: &str, name: &str, back_link: &str) -> McdbCard {
        McdbCard {
            code: code.to_string(),
            name: name.to_string(),
            pack_code: "vision".to_string(),
            position: 1,
            type_code: "hero".to_string(),
            faction_code: "hero".to_string(),
            back_link: Some(back_link.to_string()),
            hidden: Some(false),
            set_code: None,
            set_position: None,
            stage: None,
            is_unique: Some(true),
            quantity: Some(1),
        }
    }

    fn alter_ego(code: &str, name: &str, back_link: &str) -> McdbCard {
        McdbCard {
            code: code.to_string(),
            name: name.to_string(),
            pack_code: "vision".to_string(),
            position: 1,
            type_code: "alter_ego".to_string(),
            faction_code: "hero".to_string(),
            back_link: Some(back_link.to_string()),
            hidden: Some(true),
            set_code: None,
            set_position: None,
            stage: None,
            is_unique: Some(true),
            quantity: Some(1),
        }
    }

    fn plain_encounter_card(code: &str, name: &str) -> McdbCard {
        McdbCard {
            code: code.to_string(),
            name: name.to_string(),
            pack_code: "vision".to_string(),
            position: 30,
            type_code: "minion".to_string(),
            faction_code: "encounter".to_string(),
            back_link: None,
            hidden: None,
            set_code: None,
            set_position: None,
            stage: None,
            is_unique: None,
            quantity: Some(1),
        }
    }

    #[test]
    fn hidden_alter_ego_side_gets_no_independent_catalog_entry() {
        let raw = vec![
            hero("26001a", "Vision", "26001b"),
            alter_ego("26001b", "Vision", "26001a"),
        ];
        let (cards, versions) = build_cards_and_versions(&raw);

        assert_eq!(
            cards.len(),
            1,
            "the hidden alter-ego side must not become its own Card"
        );
        assert_eq!(
            versions.len(),
            1,
            "the hidden alter-ego side must not become its own CardVersion"
        );
        assert_eq!(cards[0].id, "26001a");
    }

    #[test]
    fn hero_and_encounter_cards_get_the_correct_back_group() {
        let raw = vec![
            hero("26001a", "Vision", "26001b"),
            plain_encounter_card("01140", "Ultron Drones"),
        ];
        let (cards, versions) = build_cards_and_versions(&raw);

        assert_eq!(cards.len(), 2);
        assert_eq!(versions.len(), 2);
        assert_eq!(cards[0].back_group.as_deref(), Some("player"));
        assert_eq!(cards[1].back_group.as_deref(), Some("encounter"));
    }

    #[test]
    fn a_heros_linked_alter_ego_is_surfaced_as_manifest_metadata() {
        let raw = vec![
            hero("26001a", "Vision", "26001b"),
            alter_ego("26001b", "Vision", "26001a"),
        ];
        let (cards, _) = build_cards_and_versions(&raw);

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].linked_card_code.as_deref(), Some("26001b"));
        assert_eq!(cards[0].linked_card_name.as_deref(), Some("Vision"));
        // alter_ego() fixture uses type_code "alter_ego", a PLAYER_TYPES entry.
        assert_eq!(cards[0].linked_card_back_group.as_deref(), Some("player"));
    }

    #[test]
    fn a_card_with_no_back_link_has_no_linked_card_metadata() {
        let raw = vec![plain_encounter_card("01140", "Ultron Drones")];
        let (cards, _) = build_cards_and_versions(&raw);

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].linked_card_code, None);
        assert_eq!(cards[0].linked_card_name, None);
        assert_eq!(cards[0].linked_card_back_group, None);
    }
}
