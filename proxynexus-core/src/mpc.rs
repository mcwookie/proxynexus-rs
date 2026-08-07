use crate::error::Result;
use crate::image_provider::ImageProvider;
use crate::models::Printing;
use crate::print_prep;
use async_trait::async_trait;
use image::ImageFormat;
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Cursor, Seek, Write};
use tracing::info;
use web_time::Instant;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait CardBackProvider {
    async fn fetch_card_backs(&self) -> Result<Vec<(String, Vec<u8>)>>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct MpcOptions {
    pub upscale: bool,
}

/// The 5 cardstocks mpc-autofill's order XML accepts, verified directly
/// against chilli-axe/mpc-autofill's `constants.py` `Cardstocks` enum (not
/// guessed) -- these exact strings are required by its `<stock>` element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub enum Cardstock {
    S27,
    #[default]
    S30,
    S33,
    M31,
    P10,
}

impl Cardstock {
    pub fn mpc_autofill_str(&self) -> &'static str {
        match self {
            Cardstock::S27 => "(S27) Smooth",
            Cardstock::S30 => "(S30) Standard Smooth",
            Cardstock::S33 => "(S33) Superior Smooth",
            Cardstock::M31 => "(M31) Linen",
            Cardstock::P10 => "(P10) Plastic",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct MpcAutofillOptions {
    pub stock: Cardstock,
    pub foil: bool,
}

/// One physical card slot for an mpc-autofill order: a front image and,
/// where known, a back image -- either a card's own real unique back art,
/// or the correct generic player/encounter back for that specific card
/// (never a single order-wide guess, since a mixed player+encounter print
/// job needs different generic backs on different cards). `None` only for
/// a card with no real back art and an unclassified `back_type` -- rare,
/// and left for mpc-autofill's own inert `<cardback>` fallback.
#[derive(Clone, Debug, PartialEq)]
pub struct AutofillSlot {
    pub front_filename: String,
    pub back_filename: Option<String>,
}

pub struct MpcZipOutput {
    pub zip_bytes: Vec<u8>,
    pub autofill_slots: Vec<AutofillSlot>,
}

pub async fn generate_mpc_zip(
    printings: Vec<Printing>,
    image_provider: &impl ImageProvider,
    options: MpcOptions,
    card_backs: Vec<(String, Vec<u8>)>,
    progress: Option<Box<dyn Fn(f32) + Send + Sync>>,
) -> Result<MpcZipOutput> {
    let total_images: usize = printings.iter().map(|p| 1 + p.parts.len()).sum();
    let mut processed_images = 0;

    let mut sides: HashMap<String, Vec<Printing>> = HashMap::new();
    for printing in printings {
        sides
            .entry(printing.side.clone())
            .or_default()
            .push(printing);
    }

    let mut zip_buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut zip_buffer);

    let single_side = sides.len() == 1;
    let mut written: Vec<WrittenImage> = Vec::new();

    for (side_name, side_printings) in sides {
        let folder_name = if single_side {
            "card-images".to_string()
        } else {
            format!("{}-images", side_name)
        };

        process_side(
            side_printings,
            image_provider,
            options,
            &mut zip,
            &folder_name,
            &progress,
            &mut processed_images,
            total_images,
            &mut written,
        )
        .await?;
    }

    let zip_options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (filename, bytes) in &card_backs {
        zip.start_file(filename, zip_options)?;
        zip.write_all(bytes)?;
    }

    zip.finish()?;

    let autofill_slots = build_autofill_slots(written, &card_backs);

    Ok(MpcZipOutput {
        zip_bytes: zip_buffer.into_inner(),
        autofill_slots,
    })
}

/// Everything needed, once every image in a side has actually been written
/// (so its final on-disk extension is known for certain), to later assemble
/// mpc-autofill slot assignments -- see `build_autofill_slots`.
struct WrittenImage {
    collection: String,
    card_id: String,
    variant: Option<String>,
    copy_num: u32,
    part_name: String,
    filename: String,
    /// Carried on every entry (front and back alike) for convenience, but
    /// only the front's copy is ever consulted.
    back_type: Option<String>,
}

/// Groups per-image writes back into one slot per physical card copy, and
/// fills in a generic back (matched by `back_type` against the bundled
/// card-back filenames) for any card that didn't get a real "back" part
/// written -- see `AutofillSlot`'s doc comment for why this can't just be
/// mpc-autofill's single order-wide `<cardback>` fallback.
fn build_autofill_slots(
    written: Vec<WrittenImage>,
    card_backs: &[(String, Vec<u8>)],
) -> Vec<AutofillSlot> {
    struct Group {
        front: Option<String>,
        back: Option<String>,
        back_type: Option<String>,
    }

    let mut groups: HashMap<(String, String, Option<String>, u32), Group> = HashMap::new();

    for w in written {
        let key = (w.collection, w.card_id, w.variant, w.copy_num);
        let entry = groups.entry(key).or_insert(Group {
            front: None,
            back: None,
            back_type: None,
        });
        if w.part_name == "front" {
            entry.front = Some(w.filename);
            entry.back_type = w.back_type;
        } else if w.part_name == "back" {
            entry.back = Some(w.filename);
        }
    }

    let mut slots: Vec<AutofillSlot> = groups
        .into_values()
        .filter_map(|g| {
            let front_filename = g.front?;
            let back_filename = g.back.or_else(|| {
                let back_type = g.back_type.as_deref()?;
                card_backs
                    .iter()
                    .find(|(name, _)| name.to_lowercase().contains(back_type))
                    .map(|(name, _)| name.clone())
            });
            Some(AutofillSlot {
                front_filename,
                back_filename,
            })
        })
        .collect();

    // HashMap iteration order isn't stable across runs -- sort so the
    // resulting XML is deterministic/diffable.
    slots.sort_by(|a, b| a.front_filename.cmp(&b.front_filename));
    slots
}

/// Serializes autofill slots into an mpc-autofill order XML -- schema
/// verified directly against chilli-axe/mpc-autofill's `order.py` parser
/// and a real fixture (`desktop-tool/tests/test_order.xml`), not guessed.
/// Every slot gets an explicit `<backs>` entry (see `AutofillSlot`); the
/// top-level `<cardback>` is only ever an inert fallback for the rare case
/// of a card with no real back art and an unclassified `back_type`.
pub fn generate_mpc_autofill_xml(slots: &[AutofillSlot], options: MpcAutofillOptions) -> String {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<order>\n");
    xml.push_str("  <details>\n");
    xml.push_str(&format!("    <quantity>{}</quantity>\n", slots.len()));
    xml.push_str(&format!(
        "    <stock>{}</stock>\n",
        xml_escape(options.stock.mpc_autofill_str())
    ));
    xml.push_str(&format!("    <foil>{}</foil>\n", options.foil));
    xml.push_str("  </details>\n");

    xml.push_str("  <fronts>\n");
    for (slot, s) in slots.iter().enumerate() {
        xml.push_str(&autofill_card_element(&s.front_filename, slot));
    }
    xml.push_str("  </fronts>\n");

    xml.push_str("  <backs>\n");
    for (slot, s) in slots.iter().enumerate() {
        if let Some(back) = &s.back_filename {
            xml.push_str(&autofill_card_element(back, slot));
        }
    }
    xml.push_str("  </backs>\n");

    let cardback = slots
        .iter()
        .find_map(|s| s.back_filename.as_deref())
        .unwrap_or("");
    xml.push_str(&format!(
        "  <cardback>{}</cardback>\n",
        xml_escape(cardback)
    ));

    xml.push_str("</order>\n");
    xml
}

fn autofill_card_element(filename: &str, slot: usize) -> String {
    let name = filename.rsplit('/').next().unwrap_or(filename);
    format!(
        "    <card><id>{path}</id><sourceType>Local File</sourceType><slots>{slot}</slots><name>{name}</name></card>\n",
        path = xml_escape(filename),
        name = xml_escape(name),
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[allow(clippy::too_many_arguments)]
async fn process_side<W: Write + Seek>(
    printings: Vec<Printing>,
    image_provider: &impl ImageProvider,
    options: MpcOptions,
    zip: &mut ZipWriter<W>,
    folder_name: &str,
    progress: &Option<Box<dyn Fn(f32) + Send + Sync>>,
    processed_images: &mut usize,
    total_images: usize,
    written: &mut Vec<WrittenImage>,
) -> Result<()> {
    let mut copy_counters: HashMap<(String, String, Option<String>), u32> = HashMap::new();
    let mut uniqueness_counter: u32 = 0;

    struct ImageRequest {
        printing: Printing,
        part_name: String,
        image_key: String,
        copy_num: u32,
        has_bleed: bool,
    }

    let mut requests: Vec<ImageRequest> = Vec::new();

    for printing in printings {
        let key = (
            printing.collection.clone(),
            printing.card_id.clone(),
            printing.variant.clone(),
        );
        let copy_num = copy_counters
            .entry(key)
            .and_modify(|n| *n += 1)
            .or_insert(1);

        let (front_key, front_has_bleed) = printing.mpc_image();
        let parts = printing.parts.clone();

        requests.push(ImageRequest {
            printing: printing.clone(),
            part_name: "front".to_string(),
            image_key: front_key,
            copy_num: *copy_num,
            has_bleed: front_has_bleed,
        });

        for part in parts {
            let (part_key, part_has_bleed) = part.mpc_image();
            requests.push(ImageRequest {
                printing: printing.clone(),
                part_name: part.name,
                image_key: part_key,
                copy_num: *copy_num,
                has_bleed: part_has_bleed,
            });
        }
    }

    requests.sort_by(|a, b| a.image_key.cmp(&b.image_key));

    struct CachedImage {
        key: String,
        image: image::RgbImage,
        format: ImageFormat,
    }

    let mut current_cache: Option<CachedImage> = None;

    for req in requests {
        let printing = req.printing;
        let part_name = req.part_name;
        let current_image_key = req.image_key;
        let copy_num = req.copy_num;

        uniqueness_counter += 1;
        let start = Instant::now();

        if current_cache
            .as_ref()
            .is_none_or(|c| c.key != current_image_key)
        {
            let mut image_data = image_provider.get_image_bytes(&current_image_key).await?;

            if options.upscale {
                image_data = crate::upscale_image(&image_data).await?
            }

            let image_format = image::guess_format(&image_data).unwrap_or(ImageFormat::Jpeg);
            let img = image::load_from_memory(&image_data)?;
            let bleed_image = if req.has_bleed {
                img.to_rgb8()
            } else {
                print_prep::add_bleed_border(&img)
            };

            current_cache = Some(CachedImage {
                key: current_image_key.clone(),
                image: bleed_image,
                format: image_format,
            });
        }

        let cached = current_cache.as_ref().unwrap();
        let mut final_image = cached.image.clone();
        print_prep::apply_uniqueness_marker(&mut final_image, uniqueness_counter);
        let bordered_bytes = print_prep::encode_image(final_image, cached.format)?;

        let ext = if cached.format == ImageFormat::Png {
            "png"
        } else {
            "jpg"
        };

        let variant_label = printing.variant.as_deref().unwrap_or("official");

        let filename = if part_name == "front" {
            format!(
                "{}/{}-{}-{}-{}.{}",
                folder_name, printing.card_id, variant_label, printing.collection, copy_num, ext
            )
        } else {
            format!(
                "{}/{}-{}-{}-{}-{}.{}",
                folder_name,
                printing.card_id,
                variant_label,
                printing.collection,
                copy_num,
                part_name,
                ext
            )
        };

        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file(&filename, options)?;
        zip.write_all(&bordered_bytes)?;

        written.push(WrittenImage {
            collection: printing.collection.clone(),
            card_id: printing.card_id.clone(),
            variant: printing.variant.clone(),
            copy_num,
            part_name: part_name.clone(),
            filename: filename.clone(),
            back_type: printing.back_type.clone(),
        });

        *processed_images += 1;
        if let Some(cb) = progress
            && total_images > 0
        {
            cb(*processed_images as f32 / total_images as f32);
        }

        #[cfg(not(target_arch = "wasm32"))]
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        #[cfg(target_arch = "wasm32")]
        gloo_timers::future::TimeoutFuture::new(0).await;

        info!(
            "Runtime for image {}: {:?}",
            current_image_key,
            start.elapsed()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(
        card_id: &str,
        part_name: &str,
        filename: &str,
        back_type: Option<&str>,
    ) -> WrittenImage {
        WrittenImage {
            collection: "ahlcg".to_string(),
            card_id: card_id.to_string(),
            variant: None,
            copy_num: 1,
            part_name: part_name.to_string(),
            filename: filename.to_string(),
            back_type: back_type.map(|s| s.to_string()),
        }
    }

    fn ahlcg_card_backs() -> Vec<(String, Vec<u8>)> {
        vec![
            ("ahlcg_player_back.png".to_string(), vec![]),
            ("ahlcg_encounter_back.png".to_string(), vec![]),
        ]
    }

    #[test]
    fn build_autofill_slots_uses_real_back_when_present() {
        // Carl Sanford: front is "player" classified, but has real unique
        // back art -- that real art must win over the generic player back.
        let written = vec![
            written("71034", "front", "player-images/71034.png", Some("player")),
            written(
                "71034",
                "back",
                "player-images/71034-back.png",
                Some("player"),
            ),
        ];
        let slots = build_autofill_slots(written, &ahlcg_card_backs());
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].front_filename, "player-images/71034.png");
        assert_eq!(
            slots[0].back_filename,
            Some("player-images/71034-back.png".to_string())
        );
    }

    #[test]
    fn build_autofill_slots_falls_back_to_generic_back_by_back_type() {
        let written = vec![written(
            "81029",
            "front",
            "encounter-images/81029.png",
            Some("encounter"),
        )];
        let slots = build_autofill_slots(written, &ahlcg_card_backs());
        assert_eq!(slots.len(), 1);
        assert_eq!(
            slots[0].back_filename,
            Some("ahlcg_encounter_back.png".to_string())
        );
    }

    #[test]
    fn build_autofill_slots_prefers_matching_generic_back_not_just_first() {
        // Regression check: a player-classified card must not accidentally
        // pick up the encounter generic back just because it happens to be
        // first in the card_backs list.
        let card_backs = vec![
            ("ahlcg_encounter_back.png".to_string(), vec![]),
            ("ahlcg_player_back.png".to_string(), vec![]),
        ];
        let written = vec![written(
            "71019",
            "front",
            "player-images/71019.png",
            Some("player"),
        )];
        let slots = build_autofill_slots(written, &card_backs);
        assert_eq!(
            slots[0].back_filename,
            Some("ahlcg_player_back.png".to_string())
        );
    }

    #[test]
    fn build_autofill_slots_leaves_back_none_when_unclassified_and_no_real_back() {
        let written = vec![written("99999", "front", "card-images/99999.png", None)];
        let slots = build_autofill_slots(written, &ahlcg_card_backs());
        assert_eq!(slots[0].back_filename, None);
    }

    #[test]
    fn build_autofill_slots_keeps_different_copies_as_separate_slots() {
        let written = vec![
            WrittenImage {
                copy_num: 1,
                ..written(
                    "81029",
                    "front",
                    "encounter-images/81029-1.png",
                    Some("encounter"),
                )
            },
            WrittenImage {
                copy_num: 2,
                ..written(
                    "81029",
                    "front",
                    "encounter-images/81029-2.png",
                    Some("encounter"),
                )
            },
        ];
        let slots = build_autofill_slots(written, &ahlcg_card_backs());
        assert_eq!(slots.len(), 2);
    }

    #[test]
    fn generate_mpc_autofill_xml_produces_expected_structure() {
        let slots = vec![
            AutofillSlot {
                front_filename: "player-images/71034.png".to_string(),
                back_filename: Some("player-images/71034-back.png".to_string()),
            },
            AutofillSlot {
                front_filename: "encounter-images/81029.png".to_string(),
                back_filename: Some("ahlcg_encounter_back.png".to_string()),
            },
            AutofillSlot {
                front_filename: "card-images/99999.png".to_string(),
                back_filename: None,
            },
        ];
        let xml = generate_mpc_autofill_xml(
            &slots,
            MpcAutofillOptions {
                stock: Cardstock::S33,
                foil: false,
            },
        );

        assert!(xml.contains("<quantity>3</quantity>"));
        assert!(xml.contains("<stock>(S33) Superior Smooth</stock>"));
        assert!(xml.contains("<foil>false</foil>"));

        // Every slot gets a front entry.
        assert!(xml.contains(
            "<card><id>player-images/71034.png</id><sourceType>Local File</sourceType><slots>0</slots><name>71034.png</name></card>"
        ));
        // Real back art referenced directly.
        assert!(xml.contains(
            "<card><id>player-images/71034-back.png</id><sourceType>Local File</sourceType><slots>0</slots><name>71034-back.png</name></card>"
        ));
        // Generic back referenced for the encounter card, at its own slot.
        assert!(xml.contains(
            "<card><id>ahlcg_encounter_back.png</id><sourceType>Local File</sourceType><slots>1</slots><name>ahlcg_encounter_back.png</name></card>"
        ));
        // The unclassified/no-back card (slot 2) gets a <fronts> entry but
        // no <backs> entry at all.
        let backs_section = xml
            .split("<backs>")
            .nth(1)
            .unwrap()
            .split("</backs>")
            .next()
            .unwrap();
        assert!(!backs_section.contains("<slots>2</slots>"));
        // cardback is set to whatever the first slot with a back happens to be.
        assert!(xml.contains("<cardback>player-images/71034-back.png</cardback>"));
    }

    #[test]
    fn generate_mpc_autofill_xml_escapes_special_characters() {
        let slots = vec![AutofillSlot {
            front_filename: "card-images/A & B.png".to_string(),
            back_filename: None,
        }];
        let xml = generate_mpc_autofill_xml(
            &slots,
            MpcAutofillOptions {
                stock: Cardstock::S30,
                foil: false,
            },
        );
        assert!(xml.contains("A &amp; B.png"));
        assert!(!xml.contains("A & B.png"));
    }
}
