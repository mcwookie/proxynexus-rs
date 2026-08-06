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
use crate::mpc::CardBackProvider;
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
fn back_type_for(type_code: &str, subtype_code: Option<&str>) -> Option<String> {
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

        let mut cards = Vec::with_capacity(ahdb_cards.len());
        let mut card_versions = Vec::with_capacity(ahdb_cards.len());

        for card in ahdb_cards {
            // Unlike MarvelCDB, ArkhamDB keeps both sides of a double-sided
            // card (e.g. an investigator's front/back) under one `code`, so
            // each ArkhamDB card maps 1:1 to a Card/CardVersion here. The
            // back image is picked up separately at collection-build time
            // via the `{card_id}@{pack_id}~back` filename convention.
            //
            // linked_card (when present) is a DIFFERENT thing from that --
            // a mechanically distinct card ArkhamDB never lists on its own
            // (e.g. Carl Sanford, an asset/player card up front, links to
            // 71034b, an enemy/encounter card on the back). Deliberately
            // does NOT change this card's own back_type (still purely
            // type_code-based, same as every other card) -- it only adds
            // supplementary linked_card_* info so generation output can
            // show what the physical back actually is.
            let linked_card_back_type = card
                .linked_card
                .as_ref()
                .and_then(|lc| back_type_for(&lc.type_code, None));
            cards.push(Card {
                id: card.code.clone(),
                title: card.name.clone(),
                title_normalized: normalize_title(&card.name),
                side: Some(card.faction_code.clone()),
                back_type: back_type_for(&card.type_code, card.subtype_code.as_deref()),
                linked_card_code: card.linked_card.as_ref().map(|lc| lc.code.clone()),
                linked_card_name: card.linked_card.as_ref().map(|lc| lc.name.clone()),
                linked_card_back_type,
            });

            card_versions.push(CardVersion {
                card_id: card.code,
                pack_id: card.pack_code,
                quantity: card.quantity.unwrap_or(1),
                position: Some(card.position),
            });
        }

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

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl CardBackProvider for AhlcgAdapter {
    async fn fetch_card_backs(&self) -> Result<Vec<(String, Vec<u8>)>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(vec![
                (
                    "ahlcg_player_back.png".to_string(),
                    include_bytes!("../../../assets/ahlcg_player_back.png").to_vec(),
                ),
                (
                    "ahlcg_encounter_back.png".to_string(),
                    include_bytes!("../../../assets/ahlcg_encounter_back.png").to_vec(),
                ),
            ])
        }

        #[cfg(target_arch = "wasm32")]
        {
            use futures::future::join_all;
            use gloo_net::http::Request;

            let filenames = ["ahlcg_player_back.png", "ahlcg_encounter_back.png"];

            let fetch_futures = filenames.iter().map(|filename| async move {
                let url = format!("card_backs/{}", filename);
                let response = Request::get(&url).send().await?;

                if !response.ok() {
                    return Err(crate::error::ProxyNexusError::Internal(format!(
                        "Failed to fetch {}: HTTP {}",
                        url,
                        response.status()
                    )));
                }

                let bytes = response.binary().await?;

                Ok((filename.to_string(), bytes))
            });

            let results: Vec<Result<(String, Vec<u8>)>> = join_all(fetch_futures).await;
            results.into_iter().collect()
        }
    }
}
