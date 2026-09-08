#[cfg(not(target_arch = "wasm32"))]
use crate::card_store::normalize_title;
#[cfg(not(target_arch = "wasm32"))]
use crate::catalog::{Card, CardVersion, Catalog, CatalogProvider, Pack};
#[cfg(not(target_arch = "wasm32"))]
use crate::error::Result;
use crate::card_source::{BoosterSlot, BoosterSpec};
use crate::games::GameAdapterInfo;
#[cfg(not(target_arch = "wasm32"))]
use crate::games::cohccg::models::{CohCard, CohPack};
#[cfg(not(target_arch = "wasm32"))]
use async_trait::async_trait;

/// Every City of Heroes CCG card carries the same back, so the game has one
/// back group. Type and rarity are card attributes, not reverses.
#[cfg(not(target_arch = "wasm32"))]
const COH_BACK_GROUP: &str = "card";

/// Retail booster: 11 cards, 7 common + 3 uncommon + 1 rare. Identical for
/// Arena and Secret Origins; `battle pack` cards were starter/battle-pack
/// exclusives and never appeared in boosters, so they are not a slot.
/// (archive.paragonwiki.com City of Heroes Collectible Card Game, "Retail
/// Packs".)
const COH_BOOSTER: BoosterSpec = BoosterSpec {
    slots: &[
        BoosterSlot {
            rarity: "common",
            count: 7,
        },
        BoosterSlot {
            rarity: "uncommon",
            count: 3,
        },
        BoosterSlot {
            rarity: "rare",
            count: 1,
        },
    ],
};

pub struct CohAdapter {}

impl Default for CohAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CohAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl GameAdapterInfo for CohAdapter {
    fn game_id(&self) -> &'static str {
        "cohccg"
    }

    fn game_name(&self) -> &'static str {
        "City of Heroes CCG"
    }

    fn subdomains(&self) -> Vec<&'static str> {
        vec!["cohccg"]
    }

    fn booster_spec(&self) -> Option<BoosterSpec> {
        Some(COH_BOOSTER)
    }
}

/// Turns the flat `coh_full.json` card list into catalog rows.
///
/// City of Heroes CCG has no double-sided cards -- every card is one
/// physical face with the single shared generic back (see
/// `src/games/cohccg/backs/`). Each `CohCard` maps 1:1 to one `Card` and
/// one `CardVersion` at `quantity` 1 (a CCG, not an LCG playset).
#[cfg(not(target_arch = "wasm32"))]
fn build_cards_and_versions(coh_cards: Vec<CohCard>) -> (Vec<Card>, Vec<CardVersion>) {
    let mut cards = Vec::with_capacity(coh_cards.len());
    let mut card_versions = Vec::with_capacity(coh_cards.len());

    for card in coh_cards {
        cards.push(Card {
            id: card.id.clone(),
            title: card.name.clone(),
            title_normalized: normalize_title(&card.name),
            back_group: Some(COH_BACK_GROUP.to_string()),
            rarity: Some(card.rarity),
            linked_card_code: None,
            linked_card_name: None,
            linked_card_back_group: None,
        });

        card_versions.push(CardVersion {
            card_id: card.id,
            pack_id: card.pack_code,
            quantity: 1,
            position: Some(card.number),
            api_id: None,
        });
    }

    (cards, card_versions)
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl CatalogProvider for CohAdapter {
    async fn fetch_catalog(&self) -> Result<Catalog> {
        // Both sets ship in one bundled file each -- there is no live API
        // for this out-of-print game; the data is derived from the
        // Unofficial Homecoming Wiki's card listings.
        let coh_packs: Vec<CohPack> = serde_json::from_str(include_str!("coh_packs.json"))?;
        let coh_cards: Vec<CohCard> = serde_json::from_str(include_str!("coh_full.json"))?;

        let packs: Vec<Pack> = coh_packs
            .into_iter()
            .map(|pack| Pack {
                id: pack.code,
                name: pack.name,
                date_release: pack.available,
            })
            .collect();

        let (cards, card_versions) = build_cards_and_versions(coh_cards);

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

    fn card(id: &str, pack_code: &str, number: i64, rarity: &str) -> CohCard {
        CohCard {
            id: id.to_string(),
            name: format!("Card {id}"),
            pack_code: pack_code.to_string(),
            number,
            card_type: "power".to_string(),
            rarity: rarity.to_string(),
        }
    }

    #[test]
    fn maps_id_and_pack_code_onto_card_and_version() {
        let (cards, versions) = build_cards_and_versions(vec![card("arena_011", "arena", 11, "rare")]);

        assert_eq!(cards.len(), 1);
        assert_eq!(versions.len(), 1);
        assert_eq!(cards[0].id, "arena_011");
        assert_eq!(cards[0].title, "Card arena_011");
        assert_eq!(cards[0].back_group.as_deref(), Some("card"));
        assert_eq!(cards[0].rarity.as_deref(), Some("rare"));
        assert_eq!(versions[0].card_id, "arena_011");
        assert_eq!(versions[0].pack_id, "arena");
    }

    #[test]
    fn every_card_version_is_a_single_copy() {
        let (_, versions) = build_cards_and_versions(vec![
            card("arena_001", "arena", 1, "uncommon"),
            card("so_180", "secret_origins", 180, "rare"),
        ]);

        assert!(versions.iter().all(|v| v.quantity == 1));
    }

    #[test]
    fn collector_number_becomes_version_position() {
        let (_, versions) = build_cards_and_versions(vec![card("so_144", "secret_origins", 144, "rare")]);

        assert_eq!(versions[0].position, Some(144));
    }

    #[test]
    fn bundled_catalog_parses_and_has_both_sets() {
        let packs: Vec<CohPack> =
            serde_json::from_str(include_str!("coh_packs.json")).expect("coh_packs.json parses");
        let cards: Vec<CohCard> =
            serde_json::from_str(include_str!("coh_full.json")).expect("coh_full.json parses");

        let pack_codes: Vec<&str> = packs.iter().map(|p| p.code.as_str()).collect();
        assert!(pack_codes.contains(&"arena"));
        assert!(pack_codes.contains(&"secret_origins"));

        let arena = cards.iter().filter(|c| c.pack_code == "arena").count();
        let so = cards.iter().filter(|c| c.pack_code == "secret_origins").count();
        assert_eq!(arena, 324, "Arena set size");
        assert_eq!(so, 179, "Secret Origins set size (180 numbered, #102 never printed)");

        // ids are unique across the whole game
        let mut ids: Vec<&str> = cards.iter().map(|c| c.id.as_str()).collect();
        ids.sort_unstable();
        let unique = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), unique, "card ids are unique");

        // every rarity is one of the four known values
        for c in &cards {
            assert!(
                matches!(c.rarity.as_str(), "common" | "uncommon" | "rare" | "battle pack"),
                "unexpected rarity {:?} on {}",
                c.rarity,
                c.id
            );
        }
    }
}
