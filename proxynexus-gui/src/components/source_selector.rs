use crate::components::card_list_input::CardListInput;
use async_lock::Mutex;
use dioxus::prelude::*;
use proxynexus_core::card_source::SetCopies;
use proxynexus_core::card_store::{AvailablePack, CardStore};
use proxynexus_core::db_storage::DbStorage;
use proxynexus_core::games::{get_booster_spec, get_decklist_adapter};
use std::sync::Arc;

#[derive(Clone, PartialEq, Debug)]
pub enum ActiveSource {
    Cardlist(String),
    /// A whole-set request: the set name, and how many copies of each card.
    SetName(String, SetCopies),
    DecklistUrl(String),
    /// A random booster-pack pull: set name, pack count, and RNG seed
    /// (kept in the value so a re-roll re-resolves the preview).
    Booster { set: String, packs: u32, seed: u64 },
}

/// An empty "copies" field means the retail playset; a number is a flat
/// override applied to every card in the set.
fn set_copies_of(n: Option<u32>) -> SetCopies {
    n.map_or(SetCopies::AsPrinted, SetCopies::Fixed)
}

/// A fresh random RNG seed for a booster pull.
fn fresh_seed() -> u64 {
    uuid::Uuid::new_v4().as_u64_pair().0
}

impl Default for ActiveSource {
    fn default() -> Self {
        ActiveSource::Cardlist(String::new())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum SetSortMode {
    ReleaseDate,
    Alphabetical,
}

#[derive(Props, Clone, PartialEq)]
pub struct SourceSelectorProps {
    pub active_game_id: Signal<Option<String>>,
    pub source_state: Signal<ActiveSource>,
    pub db_signal: Signal<Arc<Mutex<DbStorage>>>,
    pub on_source_changed: EventHandler<()>,
    #[props(default = None)]
    pub active_card_name: Option<Signal<Option<String>>>,
}

#[component]
pub fn SourceSelector(props: SourceSelectorProps) -> Element {
    let mut tab = use_signal(|| match &*props.source_state.peek() {
        ActiveSource::Cardlist(t) if !t.is_empty() => "list",
        ActiveSource::SetName(t, _) if !t.is_empty() => "set",
        ActiveSource::DecklistUrl(t) if !t.is_empty() => "decklist",
        ActiveSource::Booster { set, .. } if !set.is_empty() => "booster",
        _ => "list",
    });
    let db_signal = props.db_signal;
    let mut source_state = props.source_state;
    let active_game_id = props.active_game_id;

    let mut list_text = use_signal(|| {
        if let ActiveSource::Cardlist(t) = &*props.source_state.peek() {
            t.clone()
        } else {
            String::new()
        }
    });
    let mut set_name = use_signal(String::new);
    // None = each card's retail playset quantity; Some(n) = n copies of every
    // card in the set (for CCGs, where cards are singles).
    let mut set_copies = use_signal(|| None::<u32>);
    let mut decklist_url = use_signal(String::new);
    let mut booster_set = use_signal(String::new);
    let mut booster_packs = use_signal(|| 1u32);
    let mut booster_seed = use_signal(fresh_seed);
    let mut set_sort_mode = use_signal(|| SetSortMode::ReleaseDate);

    let supports_decklists =
        use_memo(move || active_game_id().is_some_and(|id| get_decklist_adapter(&id).is_some()));

    let supports_booster =
        use_memo(move || active_game_id().is_some_and(|id| get_booster_spec(&id).is_some()));

    let is_disabled = use_memo(move || active_game_id().is_none());

    let mut prev_game_id = use_signal(|| active_game_id.peek().clone());

    use_effect(move || {
        let current = active_game_id();
        if current != *prev_game_id.peek() {
            tab.set("list");
            list_text.set(String::new());
            set_name.set(String::new());
            set_copies.set(None);
            decklist_url.set(String::new());
            booster_set.set(String::new());
            booster_packs.set(1);
            booster_seed.set(fresh_seed());
            source_state.set(ActiveSource::Cardlist(String::new()));
            prev_game_id.set(current);
        }
    });

    use_effect(move || {
        if !supports_decklists() && tab() == "decklist" {
            tab.set("list");
            source_state.set(ActiveSource::Cardlist(list_text()));
        }
    });

    use_effect(move || {
        if !supports_booster() && tab() == "booster" {
            tab.set("list");
            source_state.set(ActiveSource::Cardlist(list_text()));
        }
    });

    let available_sets = use_resource(move || async move {
        let game_id = match active_game_id() {
            Some(id) => id,
            None => return Vec::new(),
        };
        let db_arc = db_signal.read().clone();
        let mut db = db_arc.lock().await;
        match CardStore::new(&mut db, game_id) {
            Ok(mut store) => {
                let packs = store.get_available_packs().await.unwrap_or_default();
                packs
                    .into_iter()
                    .filter(|pack| pack.printable > 0)
                    .collect::<Vec<_>>()
            }
            Err(_) => Vec::new(),
        }
    });

    // Re-sorts the already-fetched pack list in the GUI layer when the
    // radio buttons change, rather than re-querying the DB -- the list is
    // small (at most a few hundred packs) and already in memory.
    // get_available_packs() already returns newest-first, so the
    // ReleaseDate mode needs no reordering; only Alphabetical does.
    let sorted_sets = use_memo(move || {
        let mut sets: Vec<AvailablePack> =
            available_sets.read().as_ref().cloned().unwrap_or_default();
        if set_sort_mode() == SetSortMode::Alphabetical {
            sets.sort_by_key(|pack| pack.name.to_lowercase());
        }
        sets
    });

    let all_cards = use_resource(move || async move {
        let game_id = match active_game_id() {
            Some(id) => id,
            None => return None,
        };
        let db_arc = db_signal.read().clone();
        let mut db = db_arc.lock().await;
        match CardStore::new(&mut db, game_id) {
            Ok(mut store) => store.get_all_card_names().await.ok(),
            Err(_) => None,
        }
    });

    rsx! {
        div {
            class: "flex flex-col flex-none h-[160px] md:flex-1 md:h-auto px-4 pt-2 pb-4 w-full",

            div { class: "flex border-b border-gray-200 mb-4 shrink-0",
                button {
                    class: if tab() == "list" { "px-4 py-2 border-b-2 border-blue-600 text-blue-600 text-sm font-semibold -mb-[1px]" } else { "px-4 py-2 text-gray-500 text-sm font-medium hover:text-gray-700 border-b-2 border-transparent -mb-[1px]" },
                    onclick: move |_| {
                        if tab() != "list" {
                            props.on_source_changed.call(());
                        }
                        tab.set("list");
                        source_state.set(ActiveSource::Cardlist(list_text()));
                    },
                    "List"
                }
                button {
                    class: if tab() == "set" { "px-4 py-2 border-b-2 border-blue-600 text-blue-600 text-sm font-semibold -mb-[1px] disabled:opacity-50 disabled:cursor-not-allowed" } else { "px-4 py-2 text-gray-500 text-sm font-medium hover:text-gray-700 border-b-2 border-transparent -mb-[1px] disabled:opacity-50 disabled:cursor-not-allowed" },
                    disabled: is_disabled(),
                    onclick: move |_| {
                        if tab() != "set" {
                            props.on_source_changed.call(());
                        }
                        tab.set("set");
                        source_state
                            .set(ActiveSource::SetName(set_name(), set_copies_of(set_copies())));
                    },
                    "Set"
                }
                if supports_decklists() {
                    button {
                        class: if tab() == "decklist" { "px-4 py-2 border-b-2 border-blue-600 text-blue-600 text-sm font-semibold -mb-[1px] disabled:opacity-50 disabled:cursor-not-allowed" } else { "px-4 py-2 text-gray-500 text-sm font-medium hover:text-gray-700 border-b-2 border-transparent -mb-[1px] disabled:opacity-50 disabled:cursor-not-allowed" },
                        disabled: is_disabled(),
                        onclick: move |_| {
                            if tab() != "decklist" {
                                props.on_source_changed.call(());
                            }
                            tab.set("decklist");
                            source_state.set(ActiveSource::DecklistUrl(decklist_url()));
                        },
                        "Decklist URL"
                    }
                }
                if supports_booster() {
                    button {
                        class: if tab() == "booster" { "px-4 py-2 border-b-2 border-blue-600 text-blue-600 text-sm font-semibold -mb-[1px] disabled:opacity-50 disabled:cursor-not-allowed" } else { "px-4 py-2 text-gray-500 text-sm font-medium hover:text-gray-700 border-b-2 border-transparent -mb-[1px] disabled:opacity-50 disabled:cursor-not-allowed" },
                        disabled: is_disabled(),
                        onclick: move |_| {
                            if tab() != "booster" {
                                props.on_source_changed.call(());
                            }
                            tab.set("booster");
                            source_state.set(ActiveSource::Booster {
                                set: booster_set(),
                                packs: booster_packs().max(1),
                                seed: booster_seed(),
                            });
                        },
                        "Booster"
                    }
                }
            }

            match tab() {
                "list" => rsx! {
                    CardListInput {
                        key: "{active_game_id().unwrap_or_default()}",
                        all_cards,
                        list_text: list_text,
                        disabled: is_disabled(),
                        active_card_name: props.active_card_name,
                        oninput: move |text: String| {
                            list_text.set(text.clone());
                            source_state.set(ActiveSource::Cardlist(text));
                        }
                    }
                },
                "set" => rsx! {
                    div { class: "flex items-center gap-4 mb-2 text-sm text-gray-600",
                        label { class: "flex items-center gap-1.5 cursor-pointer",
                            input {
                                r#type: "radio",
                                name: "set-sort-mode",
                                checked: set_sort_mode() == SetSortMode::ReleaseDate,
                                onchange: move |_| set_sort_mode.set(SetSortMode::ReleaseDate),
                            }
                            "Release date"
                        }
                        label { class: "flex items-center gap-1.5 cursor-pointer",
                            input {
                                r#type: "radio",
                                name: "set-sort-mode",
                                checked: set_sort_mode() == SetSortMode::Alphabetical,
                                onchange: move |_| set_sort_mode.set(SetSortMode::Alphabetical),
                            }
                            "Alphabetical"
                        }
                    }
                    select {
                        class: "w-full p-2 border border-gray-300 rounded-md outline-none focus:ring-2 focus:ring-blue-400 bg-white text-sm disabled:bg-gray-100 disabled:text-gray-500 disabled:cursor-not-allowed",
                        disabled: is_disabled(),
                        value: "{set_name}",
                        onchange: move |evt| {
                            props.on_source_changed.call(());
                            set_name.set(evt.value());
                            source_state
                                .set(ActiveSource::SetName(evt.value(), set_copies_of(set_copies())));
                        },
                        option {
                            value: "",
                            disabled: true,
                            selected: set_name().is_empty(),
                            "Select a set..."
                        }
                        for pack in sorted_sets() {
                            option {
                                value: "{pack.name}",
                                selected: pack.name == set_name(),
                                "{pack.name}"
                            }
                        }
                    }
                    div { class: "flex items-center gap-2 mt-2 text-sm text-gray-600",
                        label { r#for: "set-copies", "Copies of each card:" }
                        input {
                            id: "set-copies",
                            r#type: "number",
                            min: "1",
                            class: "w-24 p-1.5 border border-gray-300 rounded-md outline-none focus:ring-2 focus:ring-blue-400 bg-white disabled:bg-gray-100 disabled:cursor-not-allowed",
                            disabled: is_disabled(),
                            placeholder: "playset",
                            value: set_copies().map(|n| n.to_string()).unwrap_or_default(),
                            oninput: move |evt| {
                                let parsed = evt.value().trim().parse::<u32>().ok().filter(|n| *n >= 1);
                                set_copies.set(parsed);
                                source_state
                                    .set(ActiveSource::SetName(set_name(), set_copies_of(parsed)));
                                props.on_source_changed.call(());
                            }
                        }
                        span { class: "text-gray-400", "leave blank for the retail playset" }
                    }
                },
                "decklist" => rsx! {
                    input {
                        type: "text",
                        class: "w-full p-3 border border-gray-300 rounded-md shadow-sm outline-none focus:ring-2 focus:ring-blue-400 font-mono text-sm disabled:bg-gray-100 disabled:text-gray-500 disabled:cursor-not-allowed",
                        disabled: is_disabled(),
                        placeholder: "Enter decklist URL...",
                        initial_value: "{decklist_url}",
                        oninput: move |evt| {
                            props.on_source_changed.call(());
                            decklist_url.set(evt.value());
                            source_state.set(ActiveSource::DecklistUrl(evt.value()));
                        }
                    }
                },
                "booster" => rsx! {
                    select {
                        class: "w-full p-2 border border-gray-300 rounded-md outline-none focus:ring-2 focus:ring-blue-400 bg-white text-sm disabled:bg-gray-100 disabled:text-gray-500 disabled:cursor-not-allowed",
                        disabled: is_disabled(),
                        value: "{booster_set}",
                        onchange: move |evt| {
                            props.on_source_changed.call(());
                            booster_set.set(evt.value());
                            booster_seed.set(fresh_seed());
                            source_state.set(ActiveSource::Booster {
                                set: evt.value(),
                                packs: booster_packs().max(1),
                                seed: booster_seed(),
                            });
                        },
                        option {
                            value: "",
                            disabled: true,
                            selected: booster_set().is_empty(),
                            "Select a set..."
                        }
                        for pack in sorted_sets() {
                            option {
                                value: "{pack.name}",
                                selected: pack.name == booster_set(),
                                "{pack.name}"
                            }
                        }
                    }
                    div { class: "flex items-center gap-2 mt-2 text-sm text-gray-600",
                        label { r#for: "booster-packs", "Packs:" }
                        input {
                            id: "booster-packs",
                            r#type: "number",
                            min: "1",
                            class: "w-20 p-1.5 border border-gray-300 rounded-md outline-none focus:ring-2 focus:ring-blue-400 bg-white disabled:bg-gray-100 disabled:cursor-not-allowed",
                            disabled: is_disabled(),
                            value: "{booster_packs}",
                            oninput: move |evt| {
                                let n = evt.value().trim().parse::<u32>().unwrap_or(1).max(1);
                                booster_packs.set(n);
                                source_state.set(ActiveSource::Booster {
                                    set: booster_set(),
                                    packs: n,
                                    seed: booster_seed(),
                                });
                                props.on_source_changed.call(());
                            }
                        }
                        button {
                            class: "ml-auto px-3 py-1.5 border border-gray-300 rounded-md bg-white hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: is_disabled() || booster_set().is_empty(),
                            onclick: move |_| {
                                booster_seed.set(fresh_seed());
                                source_state.set(ActiveSource::Booster {
                                    set: booster_set(),
                                    packs: booster_packs().max(1),
                                    seed: booster_seed(),
                                });
                                props.on_source_changed.call(());
                            },
                            "Re-roll"
                        }
                    }
                    p { class: "mt-1 text-xs text-gray-400",
                        "Random packs by the game's retail rarity mix. Re-roll for a new pull."
                    }
                },
                _ => rsx! { div {} }
            }
        }
    }
}
