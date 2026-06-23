use super::models::HobCard;
use crate::card_store::normalize_title;
use crate::catalog::{Card, CardVersion, Catalog, CatalogProvider, Pack};
use crate::error::Result;
use crate::games::{GameAdapterInfo, fetch_json};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};

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
        vec!["lotrlcg"]
    }
}

#[async_trait]
impl CatalogProvider for LotrLcgAdapter {
    async fn fetch_catalog(&self) -> Result<Catalog> {
        let player_cards_url = "http://hallofbeorn.com/Export/PlayerCards";
        let encounter_cards_url = "http://hallofbeorn.com/Export/EncounterCards";
        let quest_cards_url = "http://hallofbeorn.com/Export/QuestCards";

        let mut all_hob_cards: Vec<HobCard> = fetch_json(player_cards_url).await?;
        let mut encounter_cards: Vec<HobCard> = fetch_json(encounter_cards_url).await?;
        let mut quest_cards: Vec<HobCard> = fetch_json(quest_cards_url).await?;

        all_hob_cards.append(&mut encounter_cards);
        all_hob_cards.append(&mut quest_cards);

        let mut packs = Vec::new();
        let mut seen_pack_names = HashSet::new();

        for c in &all_hob_cards {
            let clean_pack_id = normalize_title(&c.card_set);
            if seen_pack_names.insert(clean_pack_id.clone()) {
                packs.push(Pack {
                    id: clean_pack_id,
                    name: c.card_set.clone(),
                    date_release: None,
                });
            }
        }

        // Pass 1: Count occurrences of (normalized_name, pack_code) to detect collisions
        let mut name_pack_counts = HashMap::new();
        for c in &all_hob_cards {
            let normalized = normalize_title(&c.title);
            let clean_pack_id = normalize_title(&c.card_set);
            *name_pack_counts
                .entry((normalized, clean_pack_id))
                .or_insert(0) += 1;
        }

        let mut cards = Vec::new();
        let mut card_versions = Vec::new();
        let mut seen_cards = HashSet::new();
        let mut seen_versions = HashSet::new();

        for c in all_hob_cards {
            let base_normalized = normalize_title(&c.title);
            let clean_pack_id = normalize_title(&c.card_set);
            let count = name_pack_counts
                .get(&(base_normalized.clone(), clean_pack_id.clone()))
                .unwrap_or(&0);

            let (title, normalized_id) = if *count > 1 {
                // Suffix with slug to guarantee uniqueness for duplicates in the same set
                let new_title = format!("{} ({})", c.title, c.slug);
                (new_title.clone(), normalize_title(&new_title))
            } else {
                (c.title.clone(), base_normalized)
            };

            let side = match c.card_type.as_str() {
                "Ally" | "Attachment" | "Contract" | "Event" | "Hero" | "Player_Side_Quest"
                | "Treasure" => "player",
                "Quest" | "Campaign" | "GenCon_Setup" | "Nightmare_Setup" => "quest",
                _ => "encounter", // Encounter_Side_Quest, Enemy, Location, Objective, Objective_Ally, Objective_Hero, Objective_Location, Ship_Enemy, Ship_Objective, Treachery, etc.
            };

            if seen_cards.insert(normalized_id.clone()) {
                cards.push(Card {
                    id: normalized_id.clone(),
                    title,
                    title_normalized: normalized_id.clone(),
                    side: Some(side.to_string()),
                });
            }

            if seen_versions.insert((normalized_id.clone(), clean_pack_id.clone())) {
                card_versions.push(CardVersion {
                    card_id: normalized_id.clone(),
                    pack_id: clean_pack_id,
                    quantity: c.quantity.unwrap_or(3),
                    position: Some(c.number),
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
