use crate::models::Printing;
use serde::{Deserialize, Serialize};

/// One row of the card-back manifest: which generic card back a given
/// printing needs when proxied, alongside enough identifying info to
/// cross-reference it against the PDF/MPC output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub card_id: String,
    pub card_title: String,
    pub collection: String,
    pub pack_id: Option<String>,
    pub variant: Option<String>,
    /// `"player"`, `"encounter"`, or `None` if the game adapter doesn't
    /// classify card backs (e.g. Netrunner, LotR LCG). Always reflects this
    /// card's own front classification -- see `linked_card_*` below for
    /// cards whose physical back is a different card entirely.
    pub back_group: Option<String>,
    /// Set when this card's back is a mechanically different card (e.g.
    /// Arkham Horror's Carl Sanford, an asset/player card up front that
    /// flips into an enemy/encounter card on the back, or a Marvel
    /// Champions hero's alter-ego) rather than either a generic back or the
    /// flip side of the same identity. `None` for the vast majority of
    /// cards.
    pub linked_card_code: Option<String>,
    pub linked_card_name: Option<String>,
    /// The linked card's own back_group (player/encounter) -- compare
    /// against `back_group` above to see whether the front and back
    /// actually need different generic backs. Kept as `linked_card_back_type`
    /// in the exported CSV/JSON for output stability across the back_group
    /// rename.
    pub linked_card_back_type: Option<String>,
    pub is_official: bool,
    pub date_release: Option<String>,
}

impl From<&Printing> for ManifestEntry {
    fn from(p: &Printing) -> Self {
        ManifestEntry {
            card_id: p.card_id.clone(),
            card_title: p.card_title.clone(),
            collection: p.collection.clone(),
            pack_id: p.pack_id.clone(),
            variant: p.variant.clone(),
            back_group: p.back_group.clone(),
            linked_card_code: p.linked_card_code.clone(),
            linked_card_name: p.linked_card_name.clone(),
            linked_card_back_type: p.linked_card_back_group.clone(),
            is_official: p.is_official,
            date_release: p.date_release.clone(),
        }
    }
}

pub fn build_manifest(printings: &[Printing]) -> Vec<ManifestEntry> {
    printings.iter().map(ManifestEntry::from).collect()
}

pub fn manifest_to_json(entries: &[ManifestEntry]) -> serde_json::Result<String> {
    serde_json::to_string_pretty(entries)
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

pub fn manifest_to_csv(entries: &[ManifestEntry]) -> String {
    let mut out = String::from(
        "card_id,card_title,collection,pack_id,variant,back_group,linked_card_code,linked_card_name,linked_card_back_type,is_official,date_release\n",
    );
    for e in entries {
        out.push_str(&csv_escape(&e.card_id));
        out.push(',');
        out.push_str(&csv_escape(&e.card_title));
        out.push(',');
        out.push_str(&csv_escape(&e.collection));
        out.push(',');
        out.push_str(&csv_escape(e.pack_id.as_deref().unwrap_or("")));
        out.push(',');
        out.push_str(&csv_escape(e.variant.as_deref().unwrap_or("")));
        out.push(',');
        out.push_str(&csv_escape(e.back_group.as_deref().unwrap_or("")));
        out.push(',');
        out.push_str(&csv_escape(e.linked_card_code.as_deref().unwrap_or("")));
        out.push(',');
        out.push_str(&csv_escape(e.linked_card_name.as_deref().unwrap_or("")));
        out.push(',');
        out.push_str(&csv_escape(
            e.linked_card_back_type.as_deref().unwrap_or(""),
        ));
        out.push(',');
        out.push_str(if e.is_official { "true" } else { "false" });
        out.push(',');
        out.push_str(&csv_escape(e.date_release.as_deref().unwrap_or("")));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CardSide;

    fn mock_printing(card_id: &str, back_group: &str) -> Printing {
        mock_printing_with_linked(card_id, back_group, None)
    }

    fn mock_printing_with_linked(
        card_id: &str,
        back_group: &str,
        linked: Option<(&str, &str, &str)>,
    ) -> Printing {
        Printing {
            card_id: card_id.to_string(),
            card_title: "Test, \"Card\"".to_string(),
            is_official: true,
            variant: None,
            front: CardSide {
                image_key: Some(format!("{}.jpg", card_id)),
                bleed_image_key: None,
            },
            backs: Vec::new(),
            collection: "core".to_string(),
            back_group: Some(back_group.to_string()),
            pack_id: Some("core-set".to_string()),
            date_release: Some("2016-01-01".to_string()),
            position: None,
            linked_card_code: linked.map(|(code, _, _)| code.to_string()),
            linked_card_name: linked.map(|(_, name, _)| name.to_string()),
            linked_card_back_group: linked.map(|(_, _, bg)| bg.to_string()),
        }
    }

    #[test]
    fn build_manifest_carries_back_group_through() {
        let printings = vec![
            mock_printing("card-1", "player"),
            mock_printing("card-2", "encounter"),
        ];
        let entries = build_manifest(&printings);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].back_group, Some("player".to_string()));
        assert_eq!(entries[1].back_group, Some("encounter".to_string()));
    }

    #[test]
    fn manifest_to_csv_escapes_commas_and_quotes() {
        let entries = build_manifest(&[mock_printing("card-1", "player")]);
        let csv = manifest_to_csv(&entries);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[1].starts_with("card-1,\"Test, \"\"Card\"\"\","));
    }

    #[test]
    fn manifest_to_json_round_trips_back_group() {
        let entries = build_manifest(&[mock_printing("card-1", "encounter")]);
        let json = manifest_to_json(&entries).unwrap();
        let parsed: Vec<ManifestEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, entries);
    }

    #[test]
    fn build_manifest_exposes_linked_card_without_changing_back_group() {
        // Carl Sanford (71034, asset/player) links to 71034b (enemy/encounter)
        // -- back_group must stay "player" (the front's own classification);
        // the linked_card_* fields carry the back's actual identity/class.
        let printings = vec![mock_printing_with_linked(
            "71034",
            "player",
            Some(("71034b", "Carl Sanford", "encounter")),
        )];
        let entries = build_manifest(&printings);
        assert_eq!(entries[0].back_group, Some("player".to_string()));
        assert_eq!(entries[0].linked_card_code, Some("71034b".to_string()));
        assert_eq!(
            entries[0].linked_card_name,
            Some("Carl Sanford".to_string())
        );
        assert_eq!(
            entries[0].linked_card_back_type,
            Some("encounter".to_string())
        );
    }

    #[test]
    fn manifest_to_csv_includes_linked_card_columns() {
        let entries = build_manifest(&[mock_printing_with_linked(
            "71034",
            "player",
            Some(("71034b", "Carl Sanford", "encounter")),
        )]);
        let csv = manifest_to_csv(&entries);
        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines[0].contains("linked_card_code,linked_card_name,linked_card_back_type"));
        assert!(lines[1].contains("71034b,Carl Sanford,encounter"));
    }
}
