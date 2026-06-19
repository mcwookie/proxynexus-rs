pub mod agot;
pub mod l5r;
pub mod netrunner;
use crate::card_source::DecklistProvider;
use crate::games::agot::adapter::AgotAdapter;
use crate::games::l5r::adapter::L5rAdapter;
use crate::games::netrunner::adapter::NetrunnerAdapter;
use crate::mpc::CardBackProvider;

pub trait GameAdapterInfo {
    fn game_id(&self) -> &'static str;
    fn game_name(&self) -> &'static str;
    fn subdomains(&self) -> Vec<&'static str> {
        vec![]
    }
}

pub fn get_game_id_by_subdomain(subdomain: &str) -> Option<&'static str> {
    let adapters: Vec<Box<dyn GameAdapterInfo>> = vec![
        Box::new(NetrunnerAdapter::new()),
        Box::new(L5rAdapter::new()),
        Box::new(AgotAdapter::new()),
    ];

    for adapter in adapters {
        if adapter.subdomains().contains(&subdomain) {
            return Some(adapter.game_id());
        }
    }
    None
}

pub fn get_decklist_adapter(game_id: &str) -> Option<Box<dyn DecklistProvider>> {
    match game_id {
        "netrunner" => Some(Box::new(NetrunnerAdapter::new())),
        "l5r" => Some(Box::new(L5rAdapter::new())),
        _ => None,
    }
}

pub fn get_card_back_adapter(game_id: &str) -> Option<Box<dyn CardBackProvider>> {
    match game_id {
        "netrunner" => Some(Box::new(NetrunnerAdapter::new())),
        "l5r" => Some(Box::new(L5rAdapter::new())),
        _ => None,
    }
}
