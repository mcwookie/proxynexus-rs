use async_trait::async_trait;
use crate::catalog::{Card, CardVersion, Catalog, CatalogProvider, Pack};
use crate::error::Result;
use crate::games::{GameAdapterInfo, fetch_json};
use crate::card_store::normalize_title;
use super::models::{RingsDbCard, RingsDbPack};

pub struct LotrLcgAdapter;

impl LotrLcgAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LotrLcgAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl GameAdapterInfo for LotrLcgAdapter {
    fn game_id(&self) -> &'static str {
        "lotrlcg"
    }

    fn game_name(&self) -> &'static str {
        "Lord of the Rings LCG"
    }

    fn subdomains(&self) -> Vec<&'static str> {
        vec!["ringsdb", "lotrlcg"]
    }
}

#[async_trait]
impl CatalogProvider for LotrLcgAdapter {
    async fn fetch_catalog(&self) -> Result<Catalog> {
        let packs_url = "https://www.ringsdb.com/api/public/packs/";
        let cards_url = "https://www.ringsdb.com/api/public/cards/";

        let ringsdb_packs: Vec<RingsDbPack> = fetch_json(packs_url).await?;
        let ringsdb_cards: Vec<RingsDbCard> = fetch_json(cards_url).await?;

        let mut packs = Vec::new();
        for p in ringsdb_packs {
            packs.push(Pack {
                id: p.get_id(),
                name: p.name,
                date_release: p.date_release,
            });
        }

        // Pass 1: Count occurrences of (normalized_name, pack_code) to detect collisions
        let mut name_pack_counts = std::collections::HashMap::new();
        for c in &ringsdb_cards {
            let normalized = normalize_title(&c.name);
            *name_pack_counts.entry((normalized, c.pack_code.clone())).or_insert(0) += 1;
        }

        let mut cards = Vec::new();
        let mut card_versions = Vec::new();
        let mut seen_cards = std::collections::HashSet::new();
        let mut seen_versions = std::collections::HashSet::new();

        for c in ringsdb_cards {
            let base_normalized = normalize_title(&c.name);
            let count = name_pack_counts.get(&(base_normalized.clone(), c.pack_code.clone())).unwrap_or(&0);
            
            let (title, normalized_id) = if *count > 1 {
                let new_title = format!("{} ({})", c.name, c.code);
                (new_title.clone(), normalize_title(&new_title))
            } else {
                (c.name.clone(), base_normalized)
            };

            if seen_cards.insert(normalized_id.clone()) {
                cards.push(Card {
                    id: normalized_id.clone(),
                    title,
                    title_normalized: normalized_id.clone(),
                    side: None, 
                });
            }

            if seen_versions.insert((normalized_id.clone(), c.pack_code.clone())) {
                card_versions.push(CardVersion {
                    card_id: normalized_id,
                    pack_id: c.pack_code.clone(),
                    quantity: c.quantity.unwrap_or(3), 
                    position: c.position,
                });
            }
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
