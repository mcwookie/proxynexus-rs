use crate::db_storage::{DbStorage, quote_sql_string};
use crate::error::{ProxyNexusError, Result};
use crate::models::Manifest;
use gluesql::FromGlueRow;
use gluesql::core::row_conversion::SelectExt;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;
use zip::ZipArchive;

#[derive(FromGlueRow)]
struct CollectionIdRow {
    id: i64,
    game_id: String,
}

#[derive(FromGlueRow)]
struct CollectionRow {
    name: String,
    version: Option<String>,
    language: Option<String>,
}

#[derive(FromGlueRow)]
struct CountRow {
    count: i64,
}

#[derive(FromGlueRow)]
struct VersionRow {
    id: String,
    card_id: String,
    pack_id: String,
}

pub struct CollectionManager<'a> {
    collections_dir: PathBuf,
    db: &'a mut DbStorage,
}

impl<'a> CollectionManager<'a> {
    pub fn new(db: &'a mut DbStorage, collections_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&collections_dir)?;

        Ok(Self {
            collections_dir,
            db,
        })
    }

    pub async fn add_collection(&mut self, pnx_path: &Path) -> Result<()> {
        if !pnx_path.exists() {
            return Err(ProxyNexusError::Internal(format!(
                "File not found: {:?}",
                pnx_path
            )));
        }

        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path();

        let file = fs::File::open(pnx_path)?;
        let mut archive = ZipArchive::new(file)?;
        archive.extract(temp_path)?;

        let manifest_path = temp_path.join("manifest.toml");
        let manifest_content = fs::read_to_string(&manifest_path)?;
        let manifest: Manifest = toml::from_str(&manifest_content)?;

        let collection_name = pnx_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ProxyNexusError::Internal("Invalid filename".into()))?
            .to_string();

        info!(
            "Adding collection: {} (v{}, {})",
            collection_name, manifest.version, manifest.language
        );

        let game_check_q = format!(
            "SELECT COUNT(*) as count FROM packs WHERE game_id = {}",
            quote_sql_string(&manifest.game)
        );
        let game_check_payload = self.db.execute(&game_check_q).await?;
        let pack_count = match game_check_payload.into_iter().next() {
            Some(p) => p
                .rows_as::<CountRow>()?
                .into_iter()
                .next()
                .map(|r| r.count)
                .unwrap_or(0),
            None => 0,
        };

        if pack_count == 0 {
            return Err(ProxyNexusError::Internal(format!(
                "No catalog found for game '{}'. Please run 'catalog update' first.",
                manifest.game
            )));
        }

        if self.collection_exists(&collection_name).await? {
            return Err(ProxyNexusError::Internal(format!(
                "Collection '{}' has already been added.",
                collection_name
            )));
        }

        let next_coll_id = self.db.get_next_id("collections").await?;

        let added_date = chrono::Utc::now().to_rfc3339();

        let insert_coll_q = format!(
            "INSERT INTO collections (id, name, game_id, version, language, added_date) VALUES ({}, {}, {}, {}, {}, '{}')",
            next_coll_id,
            quote_sql_string(&collection_name),
            quote_sql_string(&manifest.game),
            quote_sql_string(&manifest.version),
            quote_sql_string(&manifest.language),
            added_date
        );
        self.db.execute(&insert_coll_q).await?;

        let collection_id = next_coll_id;

        let versions_q = format!(
            "SELECT v.id, c.api_id as card_id, p.api_id as pack_id 
             FROM card_versions v 
             JOIN cards c ON v.card_id = c.id
             JOIN packs p ON v.pack_id = p.id 
             WHERE p.game_id = {}",
            quote_sql_string(&manifest.game)
        );
        let version_payloads = self.db.execute(&versions_q).await?;

        let mut version_map: HashMap<(String, String), String> = HashMap::new();
        if let Some(p) = version_payloads.into_iter().next() {
            let rows = p.rows_as::<VersionRow>()?;
            for row in rows {
                version_map.insert((row.card_id, row.pack_id), row.id);
            }
        }

        let collection_dir = self
            .collections_dir
            .join(&manifest.game)
            .join(collection_name.clone());
        fs::create_dir_all(&collection_dir)?;

        let src_images = temp_path.join("images");

        let mut parsed_files = Vec::new();
        for entry in fs::read_dir(&src_images)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(parsed) = Self::parse_filename(&path) {
                parsed_files.push((path, parsed));
            }
        }

        let mut parts_map: HashMap<(String, String, String), (bool, bool)> = HashMap::new();
        let mut has_front: HashMap<(String, String), bool> = HashMap::new();

        for (_, (api_id, printing, part, has_bleed)) in &parsed_files {
            let key = (api_id.clone(), printing.clone(), part.clone());
            let entry = parts_map.entry(key).or_insert((false, false));
            if *has_bleed {
                entry.1 = true;
            } else {
                entry.0 = true;
            }

            if part == "front" {
                has_front.insert((api_id.clone(), printing.clone()), true);
            } else {
                has_front
                    .entry((api_id.clone(), printing.clone()))
                    .or_insert(false);
            }
        }

        for ((api_id, printing, part), (has_non_bleed, has_bleed)) in parts_map {
            if has_bleed && !has_non_bleed {
                return Err(ProxyNexusError::Internal(format!(
                    "Validation error: Found bleed image for card '{}' ({}) part '{}', but no standard non-bleed image.",
                    api_id, printing, part
                )));
            }
        }

        for ((api_id, printing), front_exists) in has_front {
            if !front_exists {
                return Err(ProxyNexusError::Internal(format!(
                    "Validation error: Card '{}' ({}) has auxiliary parts but no 'front' image.",
                    api_id, printing
                )));
            }
        }

        self.db.execute("BEGIN").await?;

        let mut next_print_id = self.db.get_next_id("printings").await?;

        let tx_result: Result<i32> = async {
            let mut printings_added = 0;
            for (path, (api_id, parsed_printing, part, has_bleed)) in parsed_files {

                let file_name = path.file_name().unwrap().to_string_lossy();
                let file_path = format!("{}/{}/{}", manifest.game, collection_name, file_name);

                let (version_id_sql, variant_sql) =
                    if let Some(v_id) = version_map.get(&(api_id.clone(), parsed_printing.clone())) {
                        (quote_sql_string(v_id), "NULL".to_string())
                    } else {
                        ("NULL".to_string(), quote_sql_string(&parsed_printing))
                    };

                let db_card_id = format!("{}_{}", manifest.game, api_id);

                let insert_print_q = format!(
                    "INSERT INTO printings (id, collection_id, card_id, version_id, variant, file_path, part, has_bleed) VALUES ({}, {}, {}, {}, {}, {}, {}, {})",
                    next_print_id,
                    collection_id,
                    quote_sql_string(&db_card_id),
                    version_id_sql,
                    variant_sql,
                    quote_sql_string(&file_path),
                    quote_sql_string(&part),
                    if has_bleed { "TRUE" } else { "FALSE" }
                );

                self.db.execute(&insert_print_q).await?;
                next_print_id += 1;

                let dst_path = collection_dir.join(path.file_name().unwrap());
                fs::copy(&path, dst_path)?;

                printings_added += 1;
            }
            Ok(printings_added)
        }
        .await;

        let printings_added = match tx_result {
            Ok(count) => {
                self.db.execute("COMMIT").await?;
                count
            }
            Err(e) => {
                let _ = self.db.execute("ROLLBACK").await;
                return Err(ProxyNexusError::Internal(e.to_string()));
            }
        };

        info!("Added {} printings", printings_added);
        info!("Collection '{}' added successfully!", collection_name);

        Ok(())
    }

    fn parse_filename(path: &Path) -> Option<(String, String, String, bool)> {
        let mut stem = path.file_stem()?.to_str()?;

        let has_bleed = if let Some(stripped) = stem.strip_suffix(".bleed") {
            stem = stripped;
            true
        } else {
            false
        };

        let (card_id, rest) = stem.split_once('@')?;

        if rest.contains('@') {
            return None;
        }

        let (printing, part) = if let Some((pr, pt)) = rest.split_once('~') {
            if pt.contains('~') {
                return None;
            }
            (pr.to_string(), pt.to_string())
        } else {
            (rest.to_string(), "front".to_string())
        };

        Some((card_id.to_string(), printing, part, has_bleed))
    }

    pub async fn get_collections(&mut self) -> Result<Vec<(String, String, String)>> {
        let payloads = self
            .db
            .execute("SELECT name, version, language FROM collections ORDER BY name")
            .await?;

        let rows = match payloads.into_iter().next() {
            Some(p) => p.rows_as::<CollectionRow>()?,
            None => return Ok(Vec::new()),
        };

        let results = rows
            .into_iter()
            .map(|row| {
                (
                    row.name,
                    row.version.unwrap_or_default(),
                    row.language.unwrap_or_default(),
                )
            })
            .collect();

        Ok(results)
    }

    pub async fn collection_exists(&mut self, name: &str) -> Result<bool> {
        let payloads = self
            .db
            .execute(&format!(
                "SELECT COUNT(*) AS count FROM collections WHERE name = {}",
                quote_sql_string(name)
            ))
            .await?;

        let count = match payloads.into_iter().next() {
            Some(p) => p
                .rows_as::<CountRow>()?
                .into_iter()
                .next()
                .map(|row| row.count)
                .unwrap_or(0),
            None => 0,
        };
        Ok(count > 0)
    }

    pub async fn remove_collection(&mut self, collection_name: &str) -> Result<()> {
        let payloads = self
            .db
            .execute(&format!(
                "SELECT id, game_id FROM collections WHERE name = {}",
                quote_sql_string(collection_name)
            ))
            .await?;

        let (collection_id, game_id) = match payloads.into_iter().next() {
            Some(p) => p
                .rows_as::<CollectionIdRow>()?
                .into_iter()
                .next()
                .map(|row| (row.id, row.game_id))
                .ok_or_else(|| {
                    ProxyNexusError::Internal(format!("Collection '{}' not found", collection_name))
                })?,
            None => {
                return Err(ProxyNexusError::Internal(format!(
                    "Collection '{}' not found",
                    collection_name
                )));
            }
        };

        self.db.execute("BEGIN").await?;

        let tx_result: Result<()> = async {
            let del_print_q = format!(
                "DELETE FROM printings WHERE collection_id = {}",
                collection_id
            );
            self.db.execute(&del_print_q).await?;

            let del_coll_q = format!("DELETE FROM collections WHERE id = {}", collection_id);
            self.db.execute(&del_coll_q).await?;

            Ok(())
        }
        .await;

        match tx_result {
            Ok(_) => {
                self.db.execute("COMMIT").await?;
            }
            Err(e) => {
                let _ = self.db.execute("ROLLBACK").await;
                return Err(ProxyNexusError::Internal(e.to_string()));
            }
        }

        let collection_dir = self.collections_dir.join(&game_id).join(collection_name);
        if collection_dir.exists() {
            fs::remove_dir_all(&collection_dir)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_filename_variants() {
        assert_eq!(
            CollectionManager::parse_filename(Path::new("hedge_fund@system_gateway.jpg")),
            Some((
                "hedge_fund".to_string(),
                "system_gateway".to_string(),
                "front".to_string(),
                false
            ))
        );

        assert_eq!(
            CollectionManager::parse_filename(Path::new("a-legion-of-one@emerald-core-set.jpg")),
            Some((
                "a-legion-of-one".to_string(),
                "emerald-core-set".to_string(),
                "front".to_string(),
                false
            ))
        );

        assert_eq!(
            CollectionManager::parse_filename(Path::new(
                "sync_everything_everywhere@data_and_destiny~back.png"
            )),
            Some((
                "sync_everything_everywhere".to_string(),
                "data_and_destiny".to_string(),
                "back".to_string(),
                false
            ))
        );

        assert_eq!(
            CollectionManager::parse_filename(Path::new(
                "hedge_fund@system_gateway~front.bleed.jpg"
            )),
            Some((
                "hedge_fund".to_string(),
                "system_gateway".to_string(),
                "front".to_string(),
                true
            ))
        );

        assert_eq!(
            CollectionManager::parse_filename(Path::new("hedge_fund@system_gateway.bleed.png")),
            Some((
                "hedge_fund".to_string(),
                "system_gateway".to_string(),
                "front".to_string(),
                true
            ))
        );

        assert_eq!(
            CollectionManager::parse_filename(Path::new("hedge_fund~front.jpg")),
            None
        );
        assert_eq!(
            CollectionManager::parse_filename(Path::new("hedge_fund@multiple@ats.jpg")),
            None
        );
        assert_eq!(
            CollectionManager::parse_filename(Path::new("hedge_fund@dark-theme~back~extra.png")),
            None
        );
    }
}
