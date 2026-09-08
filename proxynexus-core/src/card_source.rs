use crate::card_store::CardStore;
use crate::error::Result;
use crate::games::get_decklist_adapter;
use crate::models::{Decklist, ResolvedCardRequests};
use async_trait::async_trait;

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
