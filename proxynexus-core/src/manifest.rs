use crate::models::Printing;
use serde::{Deserialize, Serialize};

/// One row of the card-back manifest: which generic card back (player or
/// encounter) a given printing needs when proxied, alongside enough
/// identifying info to cross-reference it against the PDF/MPC output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub card_id: String,
    pub card_title: String,
    pub collection: String,
    pub pack_id: Option<String>,
    pub variant: Option<String>,
    pub side: String,
    /// `"player"`, `"encounter"`, or `None` if the game adapter doesn't
    /// classify card backs (e.g. Netrunner, LotR LCG).
    pub back_type: Option<String>,
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
            side: p.side.clone(),
            back_type: p.back_type.clone(),
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
        "card_id,card_title,collection,pack_id,variant,side,back_type,is_official,date_release\n",
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
        out.push_str(&csv_escape(&e.side));
        out.push(',');
        out.push_str(&csv_escape(e.back_type.as_deref().unwrap_or("")));
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
    use crate::models::PrintingPart;

    fn mock_printing(card_id: &str, back_type: Option<&str>) -> Printing {
        Printing {
            card_id: card_id.to_string(),
            card_title: "Test, \"Card\"".to_string(),
            is_official: true,
            variant: None,
            image_key: format!("{}.jpg", card_id),
            bleed_image_key: None,
            parts: Vec::<PrintingPart>::new(),
            collection: "core".to_string(),
            side: "front".to_string(),
            pack_id: Some("core-set".to_string()),
            date_release: Some("2016-01-01".to_string()),
            back_type: back_type.map(|s| s.to_string()),
        }
    }

    #[test]
    fn build_manifest_carries_back_type_through() {
        let printings = vec![
            mock_printing("card-1", Some("player")),
            mock_printing("card-2", Some("encounter")),
            mock_printing("card-3", None),
        ];
        let entries = build_manifest(&printings);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].back_type, Some("player".to_string()));
        assert_eq!(entries[1].back_type, Some("encounter".to_string()));
        assert_eq!(entries[2].back_type, None);
    }

    #[test]
    fn manifest_to_csv_escapes_commas_and_quotes() {
        let entries = build_manifest(&[mock_printing("card-1", Some("player"))]);
        let csv = manifest_to_csv(&entries);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[1].starts_with("card-1,\"Test, \"\"Card\"\"\","));
    }

    #[test]
    fn manifest_to_json_round_trips_back_type() {
        let entries = build_manifest(&[mock_printing("card-1", Some("encounter"))]);
        let json = manifest_to_json(&entries).unwrap();
        let parsed: Vec<ManifestEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, entries);
    }
}
