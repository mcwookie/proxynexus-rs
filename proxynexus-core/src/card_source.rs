use crate::card_store::CardStore;
use crate::error::Result;
use crate::games::get_decklist_adapter;
use crate::models::{CardRequest, Decklist, ResolvedCardRequests};
use async_trait::async_trait;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

pub trait CardSource {
    #![allow(async_fn_in_trait)]
    async fn to_card_requests(&self, store: &mut CardStore<'_>) -> Result<ResolvedCardRequests>;
}

pub struct Cardlist(pub String);

/// How many copies of each card a whole-set request should emit.
///
/// LCGs ship a fixed playset per card (`card_versions.quantity`, usually
/// 3); CCGs are singles (`quantity` 1), so proxying a full CCG set as a
/// playset needs an explicit multiplier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SetCopies {
    /// One copy per `card_versions.quantity` -- the retail playset.
    #[default]
    AsPrinted,
    /// Exactly this many copies of every card in the set.
    Fixed(u32),
}

/// A whole-set request: every card in the named pack, `copies` of each.
pub struct SetName(pub String, pub SetCopies);

impl SetName {
    /// A set request that emits each card's retail playset quantity.
    pub fn as_printed(name: impl Into<String>) -> Self {
        Self(name.into(), SetCopies::AsPrinted)
    }
}

/// One rarity slot of a retail booster: `count` cards of `rarity`, each
/// drawn uniformly at random (without replacement within a single pack)
/// from that rarity's cards in the set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoosterSlot {
    pub rarity: &'static str,
    pub count: u32,
}

/// A game's retail booster-pack layout. The slot counts sum to the pack
/// size (e.g. City of Heroes CCG: 7 common + 3 uncommon + 1 rare = 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoosterSpec {
    pub slots: &'static [BoosterSlot],
}

impl BoosterSpec {
    /// Cards per pack -- the sum of every slot's count.
    pub fn pack_size(&self) -> u32 {
        self.slots.iter().map(|s| s.count).sum()
    }
}

/// A randomised booster-pack opening: `packs` packs' worth of cards from
/// the named set, drawn by the game's retail rarity slots. Cards are
/// distinct within a pack but may repeat across packs, like a real box.
pub struct BoosterPack {
    pub set: String,
    pub packs: u32,
    pub spec: BoosterSpec,
    /// `Some(seed)` for a reproducible pull; `None` seeds from the OS.
    pub seed: Option<u64>,
}

impl CardSource for BoosterPack {
    async fn to_card_requests(&self, store: &mut CardStore<'_>) -> Result<ResolvedCardRequests> {
        let pool = store.get_set_cards_by_rarity(&self.set).await?;

        let mut rng = match self.seed {
            Some(seed) => ChaCha8Rng::seed_from_u64(seed),
            None => ChaCha8Rng::from_entropy(),
        };

        let mut requests: Vec<CardRequest> = Vec::new();
        for _ in 0..self.packs {
            for slot in self.spec.slots {
                let bucket = pool.get(slot.rarity).map(Vec::as_slice).unwrap_or(&[]);
                let want = slot.count as usize;
                if bucket.len() < want {
                    tracing::warn!(
                        "set '{}' has only {} '{}' card(s); booster slot wants {}",
                        self.set,
                        bucket.len(),
                        slot.rarity,
                        want
                    );
                }
                for card in bucket.choose_multiple(&mut rng, want.min(bucket.len())) {
                    requests.push(CardRequest {
                        title: card.title.clone(),
                        id: card.id.clone(),
                        printing: Some(card.pack_id.clone()),
                        collection: None,
                        position: card.position,
                    });
                }
            }
        }

        Ok(ResolvedCardRequests {
            requests,
            not_found: Vec::new(),
        })
    }
}

pub struct DecklistUrl(pub String);

impl CardSource for DecklistUrl {
    async fn to_card_requests(&self, store: &mut CardStore<'_>) -> Result<ResolvedCardRequests> {
        let adapter = get_decklist_adapter(&store.active_game_id).ok_or_else(|| {
            crate::error::ProxyNexusError::Internal(format!(
                "The active game '{}' does not support decklist fetching.",
                store.active_game_id
            ))
        })?;
        let decklist = adapter.fetch(&self.0).await?;
        store.resolve_decklist_to_requests(&decklist).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait DecklistProvider {
    async fn fetch(&self, url: &str) -> Result<Decklist>;
}
