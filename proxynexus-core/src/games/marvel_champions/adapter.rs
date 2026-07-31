use crate::error::Result;
use crate::games::GameAdapterInfo;
#[cfg(not(target_arch = "wasm32"))]
use crate::card_store::normalize_title;
#[cfg(not(target_arch = "wasm32"))]
use crate::catalog::{Card, CardVersion, Catalog, CatalogProvider, Pack};
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
const ENCOUNTER_TYPES: &[&str] = &[
    "villain",
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
fn back_type_for(type_code: &str) -> Option<String> {
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

        let mut cards = Vec::with_capacity(mcdb_cards.len());
        let mut card_versions = Vec::with_capacity(mcdb_cards.len());

        for card in mcdb_cards {
            // Every MarvelCDB `code` — including hidden alter-ego/back-side
            // entries like Peter Parker (01001b) — becomes its own Card and
            // CardVersion. MarvelCDB already assigns each physical card face
            // its own code and image, so this 1:1 mapping lines up directly
            // with the `{card_id}@{pack_id}` image naming convention without
            // needing any `~back` part logic.
            cards.push(Card {
                id: card.code.clone(),
                title: card.name.clone(),
                title_normalized: normalize_title(&card.name),
                side: Some(card.faction_code.clone()),
                back_type: back_type_for(&card.type_code),
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
