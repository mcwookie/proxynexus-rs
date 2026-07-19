use crate::error::{ProxyNexusError, Result};
use crate::games::fetch_json;
use crate::games::marvel_champions::models::{McdbCard, McdbDecklist, McdbPack};
use crate::models::{Decklist, DecklistEntry};
use serde::Deserialize;
use std::collections::HashSet;

const BASE_URL: &str = "https://marvelcdb.com/api/public";

/// Raw shape of a single card as returned by /api/public/cards/{pack_code}.
/// Kept private and separate from McdbCard because this endpoint embeds a
/// hidden double-sided card's full data inline (under "linked_card") rather
/// than listing it as its own top-level entry -- confirmed real example:
/// hero "War Machine" (23001a) has "linked_to_code": "23001b" and a nested
/// "linked_card" object containing the complete alter-ego card "James
/// Rhodes" (23001b, hidden: true). That hidden card never appears as its
/// own entry in this listing (or the bulk endpoint) otherwise. This struct
/// captures both layers so fetch_cards_for_pack() can flatten them into
/// two separate McdbCard entries with no extra request needed.
#[derive(Debug, Clone, Deserialize)]
struct RawCard {
    code: String,
    name: String,
    pack_code: String,
    position: i64,
    type_code: String,
    faction_code: String,
    #[serde(default)]
    linked_to_code: Option<String>,
    #[serde(default)]
    linked_card: Option<Box<RawCard>>,
    #[serde(default)]
    quantity: Option<i64>,
    #[serde(default)]
    hidden: Option<bool>,
    #[serde(default)]
    set_code: Option<String>,
    #[serde(default)]
    set_position: Option<i64>,
    #[serde(default)]
    stage: Option<String>,
    #[serde(default)]
    is_unique: Option<bool>,
}

impl RawCard {
    fn into_mcdb_card(self, back_link: Option<String>) -> McdbCard {
        McdbCard {
            code: self.code,
            name: self.name,
            pack_code: self.pack_code,
            position: self.position,
            type_code: self.type_code,
            faction_code: self.faction_code,
            back_link,
            hidden: self.hidden,
            set_code: self.set_code,
            set_position: self.set_position,
            stage: self.stage,
            is_unique: self.is_unique,
            quantity: self.quantity,
        }
    }
}

/// Fetches all packs (sets/expansions). MarvelCDB returns this as a plain
/// JSON array, unlike NetrunnerDB's paginated JSON:API envelope — no
/// pagination loop needed.
pub async fn fetch_packs() -> Result<Vec<McdbPack>> {
    fetch_json(&format!("{BASE_URL}/packs/")).await
}

/// Fetches every card, correctly and completely, by iterating pack-by-pack
/// via fetch_cards_for_pack() rather than the bulk /api/public/cards/
/// endpoint.
///
/// Confirmed by direct comparison that the bulk endpoint silently drops
/// some cards: for the War Machine pack, the bulk endpoint returned 32
/// cards for pack_code "warm" while the per-pack endpoint
/// /api/public/cards/warm returned 35 — all 3 missing cards were
/// encounter-type (Minion/Treachery/Side Scheme/Obligation).
///
/// Hidden double-sided cards (e.g. a hero's alter-ego side) are handled
/// inside fetch_cards_for_pack() itself, which extracts them from the
/// visible card's embedded "linked_card" data — no extra per-card request
/// needed for those.
///
/// Still much slower than one bulk call (~60 pack requests run once during
/// `catalog update`), but the resulting catalog is actually complete.
pub async fn fetch_all_cards(packs: &[McdbPack]) -> Result<Vec<McdbCard>> {
    let mut cards = Vec::new();
    let mut seen_codes = HashSet::new();

    for pack in packs {
        let pack_cards = fetch_cards_for_pack(&pack.code).await?;
        for card in pack_cards {
            if seen_codes.insert(card.code.clone()) {
                cards.push(card);
            }
        }
    }

    Ok(cards)
}

/// Fetches cards for a single pack. Each visible card that has a hidden
/// double-sided pair (e.g. a hero's alter-ego side) carries the pair's
/// full data inline under "linked_card" — that pair never appears as its
/// own top-level entry in this listing otherwise, so it's extracted here
/// directly and flattened into a second McdbCard rather than requiring a
/// second request per hidden card.
pub async fn fetch_cards_for_pack(pack_code: &str) -> Result<Vec<McdbCard>> {
    let raw: Vec<RawCard> = fetch_json(&format!("{BASE_URL}/cards/{pack_code}")).await?;

    let mut cards = Vec::with_capacity(raw.len() * 2);
    for card in raw {
        let linked_to_code = card.linked_to_code.clone();
        let linked_card = card.linked_card.clone();
        let own_code = card.code.clone();

        cards.push(card.into_mcdb_card(linked_to_code));

        if let Some(linked) = linked_card {
            cards.push(linked.into_mcdb_card(Some(own_code)));
        }
    }

    Ok(cards)
}

/// Fetches a decklist by its MarvelCDB URL. Not yet wired into a
/// DecklistProvider impl, but ready for when adapter.rs implements one.
///
/// Accepts URLs like:
///   https://marvelcdb.com/decklist/view/12345/some-deck-name-1.0
#[allow(dead_code)]
pub async fn fetch_decklist_from_marvelcdb(url: &str) -> Result<Decklist> {
    let decklist_id = parse_marvelcdb_decklist_url(url)?;
    let api_url = format!("{BASE_URL}/decklist/{decklist_id}");

    let response: McdbDecklist = fetch_json(&api_url).await?;

    let cards = response
        .slots
        .into_iter()
        .filter_map(|(card_id, quantity)| {
            u32::try_from(quantity).ok().map(|quantity| DecklistEntry {
                card_id,
                pack_id: None,
                quantity,
            })
        })
        .collect();

    Ok(Decklist { cards })
}

#[allow(dead_code)]
fn parse_marvelcdb_decklist_url(url: &str) -> Result<String> {
    url.split("/decklist/view/")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .map(|id| id.to_string())
        .ok_or_else(|| ProxyNexusError::Internal("URL must be a MarvelCDB decklist URL".into()))
}
