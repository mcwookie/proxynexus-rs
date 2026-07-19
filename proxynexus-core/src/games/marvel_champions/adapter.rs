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
                date_release: pack.date_release,
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
