#[cfg(not(target_arch = "wasm32"))]
use super::models::HobCard;
use crate::card_source::DecklistProvider;
#[cfg(not(target_arch = "wasm32"))]
use crate::card_store::normalize_title;
#[cfg(not(target_arch = "wasm32"))]
use crate::catalog::{Card, CardVersion, Catalog, CatalogProvider, Pack};
use crate::error::Result;
use crate::games::GameAdapterInfo;
#[cfg(not(target_arch = "wasm32"))]
use crate::games::fetch_json;
use crate::games::lotrlcg::api::fetch_decklist_from_ringsdb;
use crate::models::Decklist;
use async_trait::async_trait;
#[cfg(not(target_arch = "wasm32"))]
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

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl DecklistProvider for LotrLcgAdapter {
    async fn fetch(&self, url: &str) -> Result<Decklist> {
        fetch_decklist_from_ringsdb(url).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
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

        // Build a mapping of normalized pack name to release date from RingsDB
        let mut pack_dates = HashMap::new();
        if let Ok(ringsdb_packs) = crate::games::lotrlcg::api::fetch_ringsdb_packs().await {
            for rp in ringsdb_packs {
                let clean_pack_name = rp.name.replace("ALeP - ", "").replace(".English", "");
                let clean_pack_id = normalize_title(&clean_pack_name);
                pack_dates.insert(clean_pack_id, rp.available);
            }
        }

        for c in &all_hob_cards {
            let clean_pack_id = normalize_title(&c.card_set);
            if seen_pack_names.insert(clean_pack_id.clone()) {
                packs.push(Pack {
                    id: clean_pack_id.clone(),
                    name: c.card_set.clone(),
                    date_release: pack_dates.get(&clean_pack_id).cloned(),
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
                    card_id: normalized_id,
                    pack_id: clean_pack_id,
                    quantity: c.quantity.unwrap_or(3),
                    position: Some(c.number),
                });
            }
        }

        // Fetch RingsDB cards to inject missing ALeP / custom cards not in Hall of Beorn
        if let Ok(ringsdb_cards) = crate::games::lotrlcg::api::fetch_ringsdb_catalog().await {
            for rc in ringsdb_cards {
                let normalized_id = normalize_title(&rc.name);
                let display_name = rc.pack_name.replace(".English", "");
                let clean_pack_name = rc.pack_name.replace("ALeP - ", "").replace(".English", "");
                let clean_pack_id = normalize_title(&clean_pack_name);

                if seen_pack_names.insert(clean_pack_id.clone()) {
                    packs.push(Pack {
                        id: clean_pack_id.clone(),
                        name: display_name,
                        date_release: pack_dates.get(&clean_pack_id).cloned(),
                    });
                }

                if seen_cards.insert(normalized_id.clone()) {
                    cards.push(Card {
                        id: normalized_id.clone(),
                        title: rc.name,
                        title_normalized: normalized_id.clone(),
                        side: Some("player".to_string()),
                    });
                }

                if seen_versions.insert((normalized_id.clone(), clean_pack_id.clone())) {
                    card_versions.push(CardVersion {
                        card_id: normalized_id,
                        pack_id: clean_pack_id,
                        quantity: 3, // RingsDB doesn't provide pack quantity, assume 3
                        position: Some(0), // Fallback position
                    });
                }
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
