use crate::card_source::{BoosterSlot, BoosterSpec};
#[cfg(not(target_arch = "wasm32"))]
use crate::card_store::normalize_title;
#[cfg(not(target_arch = "wasm32"))]
use crate::catalog::{Card, CardVersion, Catalog, CatalogProvider, Pack};
#[cfg(not(target_arch = "wasm32"))]
use crate::error::Result;
use crate::games::GameAdapterInfo;
#[cfg(not(target_arch = "wasm32"))]
use crate::games::meccg::models::{MeccgCard, MeccgPack};
#[cfg(not(target_arch = "wasm32"))]
use async_trait::async_trait;

/// Middle Earch CCG uses two different card backs.  The burning eye of Sauron
/// is used for Character, Resource, and Hazard cards.  The map of northwestern
/// Middle Earth is used for Site and Region cards.
#[cfg(not(target_arch = "wasm32"))]
const MECCG_BACK_GROUPS: [&str; 2] = ["eye", "map"];

/// Define which card types use the Sauron Eye back.  The other card types use the map back. 
const MECCG_BACK_SAURON_EYE: [&str; 3] = ["Character", "Resource", "Hazard"];

/// Retail booster: 15 cards, 10 common + 4 uncommon + 1 rare. 
/// MECCG is complex when it comes to pack types and distributions.
/// I need to rework this to match what was actually printed, but for now 
/// this is a reasonable approximation of the retail booster distribution.
const MECCG_BOOSTER: BoosterSpec = BoosterSpec {
    slots: &[
        BoosterSlot {
            rarity: "common",
            count: 10,
        },
        BoosterSlot {
            rarity: "uncommon",
            count: 4,
        },
        BoosterSlot {
            rarity: "rare",
            count: 1,
        },
    ],
};

pub struct MeccgAdapter {}

impl Default for MeccgAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MeccgAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl GameAdapterInfo for MeccgAdapter {
    fn game_id(&self) -> &'static str {
        "meccg"
    }

    fn game_name(&self) -> &'static str {
        "Middle Earth CCG"
    }

    fn subdomains(&self) -> Vec<&'static str> {
        vec!["meccg"]
    }

    fn booster_spec(&self) -> Option<BoosterSpec> {
        Some(MECCG_BOOSTER)
    }
}

/// Map the actual card rarities to the three standard rarities used by the catalog.
/// This should catch all the actual rarities used in the game, 
/// but if a new one is added that doesn't match the mapping, it will panic.
fn meccg_rarity_bucket(rarity: &str) -> &'static str {
    let base = rarity.split('+').next().unwrap_or(rarity);
    match base.chars().next() {
        Some('C') => "common",     // C1-C6, CA, CA2, CB, CB2
        Some('U') => "uncommon",   // U, U1-U4
        Some('R') => "rare",       // R, R1-R3
        Some('F') => "fixed",      // F1-F5 
        Some('P') => "promo",      // Not present in boosters
        _ => panic!("unmapped rarity: {rarity}"), // Rarity not found, panic.
    }
}

/// Turns the flat `meccg_full.json` card list into catalog rows.
///
/// Middle Earth CCG has no double-sided cards -- every card is one
/// physical face with one of two card backs determined by the card type (see
/// `src/games/meccg/backs/`). 
#[cfg(not(target_arch = "wasm32"))]
fn build_cards_and_versions(meccg_cards: Vec<MeccgCard>) -> (Vec<Card>, Vec<CardVersion>) {
    let mut cards = Vec::with_capacity(meccg_cards.len());
    let mut card_versions = Vec::with_capacity(meccg_cards.len());

    for card in meccg_cards {
        cards.push(Card {
            id: card.unique_id.clone(),
            title: card.label.clone(),
            title_normalized: normalize_title(&card.label),
            back_group: Some(if MECCG_BACK_SAURON_EYE.contains(&card.card_type.as_str()) {
                MECCG_BACK_GROUPS[0].to_string()
            } else {
                MECCG_BACK_GROUPS[1].to_string()
            }),
            rarity: Some(meccg_rarity_bucket(&card.rarity).to_string()),
            linked_card_code: None,
            linked_card_name: None,
            linked_card_back_group: None,
        });

        card_versions.push(CardVersion {
            card_id: card.unique_id,
            pack_id: card.pack_code,
            quantity: 1,
            position: Some(card.card_number),
            api_id: None,
        });
    }

    (cards, card_versions)
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl CatalogProvider for MeccgAdapter {
    async fn fetch_catalog(&self) -> Result<Catalog> {
        // Both sets ship in one bundled file each -- there is no live API
        // for this out-of-print game that i'm aware of; the data is derived from the
        // https://github.com/council-of-elrond-meccg/meccg-cards-database
        let meccg_packs: Vec<MeccgPack> = serde_json::from_str(include_str!("meccg_packs.json"))?;
        let meccg_cards: Vec<MeccgCard> = serde_json::from_str(include_str!("meccg_cards.json"))?;

        let packs: Vec<Pack> = meccg_packs
            .into_iter()
            .map(|pack| Pack {
                id: pack.code,
                name: pack.name,
                date_release: pack.available,
            })
            .collect();

        let (cards, card_versions) = build_cards_and_versions(meccg_cards);

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

    fn card(unique_id: &str, name: &str, label: &str, pack_code: &str, card_type: &str, alignment: &str, card_number: i64, rarity: &str) -> MeccgCard {
        MeccgCard {
            unique_id: unique_id.to_string(),
            name: name.to_string(),
            label: label.to_string(),
            pack_code: pack_code.to_string(),
            card_type: card_type.to_string(),
            alignment: alignment.to_string(),
            card_number,
            rarity: rarity.to_string(),
        }
    }

    #[test]
    fn maps_id_and_pack_code_onto_card_and_version() {
        let (cards, versions) =
            build_cards_and_versions(vec![card("DM-99", "Waylaid, Wounded, and Orc-dragged", "Waylaid, Wounded, and Orc-dragged", "dark-minions", "Hazard", "Neutral", 99, "U2")]);

        assert_eq!(cards.len(), 1);
        assert_eq!(versions.len(), 1);
        assert_eq!(cards[0].id, "DM-99");
        assert_eq!(cards[0].title, "Waylaid, Wounded, and Orc-dragged");
        assert_eq!(cards[0].back_group.as_deref(), Some("eye"));
        assert_eq!(cards[0].rarity.as_deref(), Some("uncommon"));
        assert_eq!(versions[0].card_id, "DM-99");
        assert_eq!(versions[0].pack_id, "dark-minions");
    }

    #[test]
    fn every_card_version_is_a_single_copy() {
        let (_, versions) = build_cards_and_versions(vec![
            card("DM-99", "Waylaid, Wounded, and Orc-dragged", "Waylaid, Wounded, and Orc-dragged", "dark-minions", "Hazard", "Neutral", 99, "U2"),
            card("LE-74", "Giant", "Giant (the-lidless-eye)", "the-lidless-eye", "Hazard", "Neutral", 74, "C3"),
        ]);

        assert!(versions.iter().all(|v| v.quantity == 1));
    }

    #[test]
    fn collector_number_becomes_version_position() {
        let (_, versions) =
            build_cards_and_versions(vec![card("DM-99", "Waylaid, Wounded, and Orc-dragged", "Waylaid, Wounded, and Orc-dragged", "dark-minions", "Hazard", "Neutral", 99, "U2")]);

        assert_eq!(versions[0].position, Some(99));
    }

    #[test]
    fn bundled_catalog_parses_and_has_both_sets() {
        let packs: Vec<MeccgPack> =
            serde_json::from_str(include_str!("meccg_packs.json")).expect("meccg_packs.json parses");
        let cards: Vec<MeccgCard> =
            serde_json::from_str(include_str!("meccg_cards.json")).expect("meccg_cards.json parses");

        let pack_codes: Vec<&str> = packs.iter().map(|p| p.code.as_str()).collect();
        assert!(pack_codes.contains(&"dark-minions"));
        assert!(pack_codes.contains(&"the-lidless-eye"));

        let dark_minions = cards.iter().filter(|c| c.pack_councommonde == "dark-minions").count();
        let the_lidless_eye = cards
            .iter()
            .filter(|c| c.pack_code == "the-lidless-eye")
            .count();
        assert_eq!(dark_minions, 183, "Dark Minions set size");
        assert_eq!(
            the_lidless_eye, 419,
            "The Lidless Eye set size (419 numbered)"
        );

        // ids are unique across the whole game
        let mut ids: Vec<&str> = cards.iter().map(|c| c.unique_id.as_str()).collect();
        ids.sort_unstable();
        let unique = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), unique, "card ids are unique");

        // forces every real rarity code through the bucket fn; panics (failing the test)
        // if any card in the bundled data has an unrecognized rarity
        for c in &cards {
            meccg_rarity_bucket(&c.rarity);
        }
    }
}
