use crate::card_backs::{self, CardBack};
use crate::error::Result;
use crate::file_naming::{back_index, back_label};
use crate::image_provider::ImageProvider;
use crate::models::{BleedPreference, Printing, SourceImage};
use crate::print_prep;
use image::ImageFormat;
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Cursor, Seek, Write};
use tracing::info;
use web_time::Instant;
use zip::ZipWriter;
use zip::read::ZipArchive;
use zip::write::SimpleFileOptions;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct MpcOptions {
    pub upscale: bool,
}

/// Which MakePlayingCards cardstock an autofill order should print on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MpcAutofillOptions {
    pub stock: Cardstock,
    pub foil: bool,
}

/// One physical card in an mpc-autofill order: the zip-relative filename of
/// its front image, and its back's if it has one (either a real, card-
/// specific back, or the generic back for its back_group).
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
    game_id: &str,
    printings: Vec<Printing>,
    image_provider: &impl ImageProvider,
    options: MpcOptions,
    card_backs: Vec<(&'static CardBack, Vec<u8>)>,
    progress: Option<Box<dyn Fn(f32) + Send + Sync>>,
) -> Result<MpcZipOutput> {
    let total_images: usize = printings.iter().map(|p| 1 + p.backs.len()).sum();
    let mut processed_images = 0;

    // A card whose adapter can't classify it (`back_group: None`) has no
    // generic back to look up at all -- kept as its own `None` group here
    // rather than folded into a sentinel string, so the lookup below is
    // skipped outright instead of depending on no real game ever shipping
    // a back_group that happens to collide with a made-up placeholder.
    let mut back_groups: HashMap<Option<String>, Vec<Printing>> = HashMap::new();
    for printing in printings {
        back_groups
            .entry(printing.back_group.clone())
            .or_default()
            .push(printing);
    }

    let mut zip_buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut zip_buffer);
    let mut autofill_slots = Vec::new();

    let single_group = back_groups.len() == 1;

    for (group_name, group_printings) in back_groups {
        let group_label = group_name.as_deref().unwrap_or("unclassified");
        let folder_name = if single_group {
            "card-images".to_string()
        } else {
            format!("{}-images", group_label)
        };

        let generic_back = group_name
            .as_deref()
            .and_then(|group| card_backs::card_back(game_id, group, None))
            .map(|cb| cb.file.to_string());

        let slots = process_back_group(
            group_printings,
            image_provider,
            options,
            &mut zip,
            &folder_name,
            &progress,
            &mut processed_images,
            total_images,
        )
        .await?;

        autofill_slots.extend(slots.into_iter().map(|mut slot| {
            if slot.back_filename.is_none() {
                slot.back_filename = generic_back.clone();
            }
            slot
        }));
    }

    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (back, bytes) in card_backs {
        let prepared = if back.has_bleed {
            bytes
        } else {
            let format = image::guess_format(&bytes).unwrap_or(ImageFormat::Jpeg);
            let img = image::load_from_memory(&bytes)?;
            print_prep::encode_image(print_prep::add_mpc_bleed_border(&img), format)?
        };

        zip.start_file(back.file, options)?;
        zip.write_all(&prepared)?;
    }

    zip.finish()?;
    Ok(MpcZipOutput {
        zip_bytes: zip_buffer.into_inner(),
        autofill_slots,
    })
}

/// Merges extra named entries into an already-built zip without
/// recompressing its existing contents.
pub fn append_files_to_zip(zip_bytes: Vec<u8>, extra_files: &[(&str, &[u8])]) -> Result<Vec<u8>> {
    let archive = ZipArchive::new(Cursor::new(zip_bytes))?;
    let mut buffer = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(&mut buffer);
    writer.merge_archive(archive)?;

    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, contents) in extra_files {
        writer.start_file(*name, options)?;
        writer.write_all(contents)?;
    }
    writer.finish()?;
    Ok(buffer.into_inner())
}

pub fn generate_mpc_autofill_xml(slots: &[AutofillSlot], options: MpcAutofillOptions) -> String {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<order>\n  <details>\n");
    xml.push_str(&format!("    <quantity>{}</quantity>\n", slots.len()));
    xml.push_str(&format!(
        "    <stock>{}</stock>\n",
        xml_escape(options.stock.mpc_autofill_str())
    ));
    xml.push_str(&format!("    <foil>{}</foil>\n", options.foil));
    xml.push_str("  </details>\n  <fronts>\n");
    for (slot, s) in slots.iter().enumerate() {
        xml.push_str(&autofill_card_element(&s.front_filename, slot));
    }
    xml.push_str("  </fronts>\n  <backs>\n");
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
async fn process_back_group<W: Write + Seek>(
    printings: Vec<Printing>,
    image_provider: &impl ImageProvider,
    options: MpcOptions,
    zip: &mut ZipWriter<W>,
    folder_name: &str,
    progress: &Option<Box<dyn Fn(f32) + Send + Sync>>,
    processed_images: &mut usize,
    total_images: usize,
) -> Result<Vec<AutofillSlot>> {
    let mut copy_counters: HashMap<(String, String, Option<String>), u32> = HashMap::new();
    let mut uniqueness_counter: u32 = 0;

    #[derive(Default)]
    struct SlotBuilder {
        front: Option<String>,
        backs: Vec<(u32, String)>,
    }
    let mut slot_builders: HashMap<(String, String, Option<String>, u32), SlotBuilder> =
        HashMap::new();

    struct ImageRequest {
        printing: Printing,
        side_name: String,
        source: SourceImage,
        copy_num: u32,
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

        let backs = printing.backs.clone();

        if let Some(source) = printing.front.image(BleedPreference::Bleed) {
            requests.push(ImageRequest {
                printing: printing.clone(),
                side_name: "front".to_string(),
                source,
                copy_num: *copy_num,
            });
        }

        for (offset, back) in backs.iter().enumerate() {
            if let Some(source) = back.image(BleedPreference::Bleed) {
                requests.push(ImageRequest {
                    printing: printing.clone(),
                    side_name: back_label(offset as u32 + 1),
                    source,
                    copy_num: *copy_num,
                });
            }
        }
    }

    requests.sort_by(|a, b| a.source.key.cmp(&b.source.key));

    struct CachedImage {
        key: String,
        image: image::RgbImage,
        format: ImageFormat,
    }

    let mut current_cache: Option<CachedImage> = None;

    for req in requests {
        let printing = req.printing;
        let side_name = req.side_name;
        let current_image_key = req.source.key;
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
            let bleed_image = if req.source.has_bleed {
                img.to_rgb8()
            } else {
                print_prep::add_mpc_bleed_border(&img)
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

        let filename = if side_name == "front" {
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
                side_name,
                ext
            )
        };

        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file(&filename, options)?;
        zip.write_all(&bordered_bytes)?;

        let slot_key = (
            printing.collection.clone(),
            printing.card_id.clone(),
            printing.variant.clone(),
            copy_num,
        );
        let builder = slot_builders.entry(slot_key).or_default();
        if side_name == "front" {
            builder.front = Some(filename.clone());
        } else if let Some(index) = back_index(&side_name) {
            builder.backs.push((index, filename.clone()));
        }

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

    let mut slots: Vec<AutofillSlot> = Vec::new();
    for builder in slot_builders.into_values() {
        let Some(front) = builder.front else {
            continue;
        };
        if builder.backs.is_empty() {
            slots.push(AutofillSlot {
                front_filename: front,
                back_filename: None,
            });
        } else {
            let mut backs = builder.backs;
            backs.sort_by_key(|(index, _)| *index);
            for (_, back_filename) in backs {
                slots.push(AutofillSlot {
                    front_filename: front.clone(),
                    back_filename: Some(back_filename),
                });
            }
        }
    }

    Ok(slots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CardSide;

    struct MockImageProvider;

    impl ImageProvider for MockImageProvider {
        async fn get_image_bytes(&self, _key: &str) -> Result<Vec<u8>> {
            let img = image::RgbImage::from_pixel(1, 1, image::Rgb([255, 255, 255]));
            let mut bytes = Vec::new();
            image::DynamicImage::ImageRgb8(img)
                .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
                .unwrap();
            Ok(bytes)
        }
    }

    fn side(key: &str) -> CardSide {
        CardSide {
            image_key: Some(key.to_string()),
            bleed_image_key: None,
        }
    }

    fn printing(card_id: &str, backs: Vec<CardSide>) -> Printing {
        Printing {
            card_id: card_id.to_string(),
            card_title: card_id.to_string(),
            is_official: true,
            variant: None,
            front: side(&format!("{}.png", card_id)),
            backs,
            collection: "test-collection".to_string(),
            back_group: Some("card".to_string()),
            pack_id: None,
            date_release: None,
            position: None,
            linked_card_code: None,
            linked_card_name: None,
            linked_card_back_group: None,
        }
    }

    #[tokio::test]
    async fn a_printing_with_no_real_back_gets_the_generic_card_back() {
        // "agot" ships one back_group ("card") with one physical back file,
        // so any printing without its own real back should be paired with
        // exactly that file.
        let card_backs = card_backs::fetch_card_backs("agot").await.unwrap();
        let expected_back = card_backs[0].0.file;

        let output = generate_mpc_zip(
            "agot",
            vec![printing("card-a", Vec::new())],
            &MockImageProvider,
            MpcOptions::default(),
            card_backs,
            None,
        )
        .await
        .unwrap();

        assert_eq!(output.autofill_slots.len(), 1);
        assert_eq!(
            output.autofill_slots[0].back_filename.as_deref(),
            Some(expected_back)
        );
    }

    #[tokio::test]
    async fn a_printing_with_a_real_back_uses_its_own_image_not_the_generic_one() {
        let card_backs = card_backs::fetch_card_backs("agot").await.unwrap();

        let output = generate_mpc_zip(
            "agot",
            vec![printing("card-b", vec![side("card-b-back.png")])],
            &MockImageProvider,
            MpcOptions::default(),
            card_backs,
            None,
        )
        .await
        .unwrap();

        assert_eq!(output.autofill_slots.len(), 1);
        let back_filename = output.autofill_slots[0].back_filename.as_deref().unwrap();
        assert!(
            back_filename.contains("card-b"),
            "expected the real back's own filename, got {back_filename}"
        );
    }

    #[tokio::test]
    async fn an_unclassified_card_gets_no_generic_back_but_still_generates() {
        // back_group: None (an adapter that couldn't classify the card at
        // all) must not crash the lookup or the zip -- it just means no
        // generic back is available for it.
        let card_backs = card_backs::fetch_card_backs("agot").await.unwrap();
        let mut unclassified = printing("card-c", Vec::new());
        unclassified.back_group = None;

        let output = generate_mpc_zip(
            "agot",
            vec![unclassified],
            &MockImageProvider,
            MpcOptions::default(),
            card_backs,
            None,
        )
        .await
        .unwrap();

        assert_eq!(output.autofill_slots.len(), 1);
        assert_eq!(output.autofill_slots[0].back_filename, None);
    }

    #[test]
    fn autofill_xml_pairs_each_front_with_its_own_back_by_slot_index() {
        let slots = vec![
            AutofillSlot {
                front_filename: "player-images/a-official-core-1.jpg".into(),
                back_filename: Some("player_original.jpg".into()),
            },
            AutofillSlot {
                front_filename: "player-images/b-official-core-1.jpg".into(),
                back_filename: None,
            },
        ];
        let xml = generate_mpc_autofill_xml(&slots, MpcAutofillOptions::default());

        assert!(xml.contains("<slots>0</slots><name>a-official-core-1.jpg</name>"));
        assert!(xml.contains("<slots>0</slots><name>player_original.jpg</name>"));
        assert!(!xml.contains("<slots>1</slots><name>player_original"));
        assert!(xml.contains("<cardback>player_original.jpg</cardback>"));
    }
}
