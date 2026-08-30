use crate::card_source::DecklistProvider;
#[cfg(not(target_arch = "wasm32"))]
use crate::card_store::normalize_title;
#[cfg(not(target_arch = "wasm32"))]
use crate::catalog::{Card, CardVersion, Catalog, CatalogProvider, Pack};
use crate::error::Result;
use crate::games::GameAdapterInfo;
use crate::games::ahlcg::api::fetch_decklist_from_arkhamdb;
#[cfg(not(target_arch = "wasm32"))]
use crate::games::ahlcg::api::{fetch_all_cards, fetch_packs};
use crate::models::Decklist;
use async_trait::async_trait;

pub struct AhlcgAdapter {}

impl Default for AhlcgAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AhlcgAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl GameAdapterInfo for AhlcgAdapter {
    fn game_id(&self) -> &'static str {
        "ahlcg"
    }

    fn game_name(&self) -> &'static str {
        "Arkham Horror: The Card Game"
    }

    fn subdomains(&self) -> Vec<&'static str> {
        vec!["ahlcg"]
    }
}

// Which generic card back a card needs, classified by `type_code` rather
// than `faction_code`/side -- confirmed real cards where they'd disagree:
// 10 cards are type_code "asset" (usable in a player deck) but
// faction_code "mythos" (e.g. "The Face", "The Muscle", recruitable allies
// found via an encounter set) -- a faction-based guess would wrongly call
// these encounter-back. Checked the reverse direction too (encounter-type
// cards with a player-class faction): zero cases. `type_code` values
// confirmed exhaustively against a full-catalog sample (not just Core
// Set) as of this writing; anything not in either list below is left
// unclassified (`None`) rather than guessed.
#[cfg(not(target_arch = "wasm32"))]
const PLAYER_TYPES: &[&str] = &["investigator", "asset", "event", "skill"];
#[cfg(not(target_arch = "wasm32"))]
const ENCOUNTER_TYPES: &[&str] = &[
    "enemy",
    "enemy_location",
    "treachery",
    "agenda",
    "act",
    "location",
    "scenario",
    "story",
    "key",
];

#[cfg(not(target_arch = "wasm32"))]
fn back_group_for(type_code: &str, subtype_code: Option<&str>) -> Option<String> {
    // Weakness cards are drawn from -- and shuffled back into -- the
    // investigator's own deck, so they carry the PLAYER card back
    // regardless of type_code, even for "enemy"/"treachery" weaknesses
    // (e.g. Mob Goons, 08003) that resolve as an "encounter cardtype" card
    // per the Rules Reference once drawn. That resolution-mechanics
    // classification is not the same axis as the physical print -- see
    // `AhdbCard::subtype_code`'s doc comment for the confirming evidence.
    if matches!(subtype_code, Some("weakness") | Some("basicweakness")) {
        return Some("player".to_string());
    }
    if PLAYER_TYPES.contains(&type_code) {
        Some("player".to_string())
    } else if ENCOUNTER_TYPES.contains(&type_code) {
        Some("encounter".to_string())
    } else {
        None
    }
}

/// Turns the flat, per-pack-fetched `AhdbCard` list into catalog rows.
///
/// ArkhamDB keeps both sides of a double-sided card (e.g. an investigator's
/// front/back) under one `code`, so each ArkhamDB card maps 1:1 to a
/// Card/CardVersion here. The back image is picked up separately at
/// collection-build time via the `{card_id}@{pack_id}~back` filename
/// convention.
///
/// `card.linked_card` (a mechanically distinct card ArkhamDB never lists on
/// its own, e.g. Carl Sanford flipping into an enemy on the back) has no
/// analogue in the back_group/CardSide model, so it's carried here as
/// fork-only metadata for `manifest.rs` instead -- purely informational,
/// since the back *image* already resolves independently via the filename
/// convention above regardless of this.
#[cfg(not(target_arch = "wasm32"))]
fn build_cards_and_versions(
    ahdb_cards: Vec<crate::games::ahlcg::models::AhdbCard>,
) -> (Vec<Card>, Vec<CardVersion>) {
    let mut cards = Vec::with_capacity(ahdb_cards.len());
    let mut card_versions = Vec::with_capacity(ahdb_cards.len());

    for card in ahdb_cards {
        let linked = card.linked_card.as_deref();
        cards.push(Card {
            id: card.code.clone(),
            title: card.name.clone(),
            title_normalized: normalize_title(&card.name),
            back_group: back_group_for(&card.type_code, card.subtype_code.as_deref()),
            linked_card_code: linked.map(|l| l.code.clone()),
            linked_card_name: linked.map(|l| l.name.clone()),
            linked_card_back_group: linked.and_then(|l| back_group_for(&l.type_code, None)),
        });

        card_versions.push(CardVersion {
            card_id: card.code,
            pack_id: card.pack_code,
            quantity: card.quantity.unwrap_or(1),
            position: Some(card.position),
            api_id: None,
        });
    }

    (cards, card_versions)
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl CatalogProvider for AhlcgAdapter {
    async fn fetch_catalog(&self) -> Result<Catalog> {
        let ahdb_packs = fetch_packs().await?;
        // fetch_all_cards() fetches per-pack rather than the bulk
        // /api/public/cards/ endpoint -- confirmed the bulk endpoint is
        // badly incomplete (see api.rs docs). Slower (~115 requests instead
        // of 1) but the catalog is actually complete.
        let ahdb_cards = fetch_all_cards(&ahdb_packs).await?;

        let packs: Vec<Pack> = ahdb_packs
            .into_iter()
            .map(|pack| Pack {
                id: pack.code,
                name: pack.name,
                date_release: pack.available,
            })
            .collect();

        let (cards, card_versions) = build_cards_and_versions(ahdb_cards);

        Ok(Catalog {
            game_id: self.game_id().to_string(),
            display_name: self.game_name().to_string(),
            packs,
            cards,
            card_versions,
        })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl DecklistProvider for AhlcgAdapter {
    async fn fetch(&self, url: &str) -> Result<Decklist> {
        fetch_decklist_from_arkhamdb(url).await
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::games::ahlcg::models::{AhdbCard, AhdbLinkedCard};

    fn card(code: &str, name: &str, type_code: &str, subtype_code: Option<&str>) -> AhdbCard {
        AhdbCard {
            code: code.to_string(),
            name: name.to_string(),
            pack_code: "core".to_string(),
            position: 1,
            type_code: type_code.to_string(),
            faction_code: "neutral".to_string(),
            quantity: Some(1),
            double_sided: None,
            linked_to_code: None,
            linked_card: None,
            subtype_code: subtype_code.map(|s| s.to_string()),
        }
    }

    fn linked_card(
        code: &str,
        name: &str,
        type_code: &str,
        linked_code: &str,
        linked_name: &str,
        linked_type_code: &str,
    ) -> AhdbCard {
        AhdbCard {
            linked_to_code: Some(linked_code.to_string()),
            linked_card: Some(Box::new(AhdbLinkedCard {
                code: linked_code.to_string(),
                name: linked_name.to_string(),
                type_code: linked_type_code.to_string(),
            })),
            ..card(code, name, type_code, None)
        }
    }

    #[test]
    fn player_and_encounter_cards_get_the_correct_back_group() {
        let raw = vec![
            card("01006", "Roland Banks", "investigator", None),
            card("01121", "Ghoul Priest", "enemy", None),
        ];
        let (cards, versions) = build_cards_and_versions(raw);

        assert_eq!(cards.len(), 2);
        assert_eq!(versions.len(), 2);
        assert_eq!(cards[0].back_group.as_deref(), Some("player"));
        assert_eq!(cards[1].back_group.as_deref(), Some("encounter"));
    }

    #[test]
    fn a_card_linked_to_a_mechanically_different_card_is_surfaced_as_manifest_metadata() {
        // Carl Sanford (asset/player up front) flips into an enemy/encounter
        // card on the back -- a real ArkhamDB case, not a hypothetical.
        let raw = vec![linked_card(
            "71034",
            "Carl Sanford",
            "asset",
            "71034b",
            "Carl Sanford",
            "enemy",
        )];
        let (cards, _) = build_cards_and_versions(raw);

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].back_group.as_deref(), Some("player"));
        assert_eq!(cards[0].linked_card_code.as_deref(), Some("71034b"));
        assert_eq!(cards[0].linked_card_name.as_deref(), Some("Carl Sanford"));
        assert_eq!(
            cards[0].linked_card_back_group.as_deref(),
            Some("encounter")
        );
    }

    #[test]
    fn a_card_with_no_linked_card_has_no_linked_card_metadata() {
        let raw = vec![card("01006", "Roland Banks", "investigator", None)];
        let (cards, _) = build_cards_and_versions(raw);

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].linked_card_code, None);
        assert_eq!(cards[0].linked_card_name, None);
        assert_eq!(cards[0].linked_card_back_group, None);
    }
}
