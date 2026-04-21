use crate::error::{ProxyNexusError, Result};
use async_once_cell::OnceCell;
use burn::tensor::Tensor;
use burn::tensor::backend::Backend;
use tracing::info;

#[derive(Debug)]
struct InferenceState {
    model: model::Model<WgpuBackend>,
    device: <WgpuBackend as Backend>::Device,
}

static STATE: OnceCell<InferenceState> = OnceCell::new();

const SCALE: usize = 4; // needs to match the scale value used when creating the onnx file
const TILE_SIZE: usize = 400;
const TILE_PAD: usize = 10;

type WgpuBackend = burn_wgpu::Wgpu<f32, i32>;

pub mod model {
    include!(concat!(env!("OUT_DIR"), "/realesr-general-x4v3.rs"));
}

#[cfg(target_arch = "wasm32")]
pub fn is_webgpu_available() -> bool {
    web_sys::window().is_some_and(|w| {
        let nav = w.navigator();
        js_sys::Reflect::has(&nav, &"gpu".into()).unwrap_or(false)
    })
}

pub async fn upscale_image(bytes: &[u8]) -> Result<Vec<u8>> {
    let img =
        image::load_from_memory(bytes).map_err(|e| ProxyNexusError::Internal(e.to_string()))?;
    let img_rgb = img.to_rgb8();

    let state = STATE
        .get_or_try_init(async {
            let device = burn_wgpu::WgpuDevice::default();

            #[cfg(target_arch = "wasm32")]
            {
                info!("Initializing wgpu runtime...");
                burn_wgpu::init_setup_async::<burn_wgpu::graphics::WebGpu>(
                    &device,
                    Default::default(),
                )
                .await;
            }

            info!("Loading model...");
            let weights = fetch_model_weights().await?;
            let model = model::Model::<WgpuBackend>::from_bytes(
                burn::tensor::Bytes::from_bytes_vec(weights),
                &device,
            );

            info!("Model and device loaded successfully.");
            Ok::<InferenceState, ProxyNexusError>(InferenceState { model, device })
        })
        .await?;

    let out_img = process_tiled(&img_rgb, &state.model, &state.device).await?;
    finalize_upscale_img(out_img)
}

async fn process_tiled<B: Backend>(
    img_rgb: &image::RgbImage,
    model: &model::Model<B>,
    device: &B::Device,
) -> Result<image::RgbImage> {
    let width = img_rgb.width() as usize;
    let height = img_rgb.height() as usize;
    let out_width = width * SCALE;
    let out_height = height * SCALE;
    let mut out_img = image::RgbImage::new(out_width as u32, out_height as u32);

    for in_y in (0..height).step_by(TILE_SIZE) {
        for in_x in (0..width).step_by(TILE_SIZE) {
            let in_w = (TILE_SIZE).min(width - in_x);
            let in_h = (TILE_SIZE).min(height - in_y);

            let pad_left = (in_x).min(TILE_PAD);
            let pad_top = (in_y).min(TILE_PAD);
            let pad_right = (width - (in_x + in_w)).min(TILE_PAD);
            let pad_bottom = (height - (in_y + in_h)).min(TILE_PAD);

            let tile_x = in_x - pad_left;
            let tile_y = in_y - pad_top;
            let tile_w = in_w + pad_left + pad_right;
            let tile_h = in_h + pad_top + pad_bottom;

            let mut planar_pixels = vec![0.0; 3 * tile_w * tile_h];
            let area = tile_w * tile_h;

            for ty in 0..tile_h {
                for tx in 0..tile_w {
                    let pixel = img_rgb.get_pixel((tile_x + tx) as u32, (tile_y + ty) as u32);
                    let i = ty * tile_w + tx;
                    planar_pixels[i] = pixel[0] as f32 / 255.0;
                    planar_pixels[area + i] = pixel[1] as f32 / 255.0;
                    planar_pixels[area * 2 + i] = pixel[2] as f32 / 255.0;
                }
            }

            let out_data = run_inference(model, device, planar_pixels, tile_w, tile_h).await?;

            let out_tile_w = tile_w * SCALE;
            let out_area = out_tile_w * tile_h * SCALE;

            let out_pad_left = pad_left * SCALE;
            let out_pad_top = pad_top * SCALE;
            let out_in_w = in_w * SCALE;
            let out_in_h = in_h * SCALE;

            for ty in 0..out_in_h {
                for tx in 0..out_in_w {
                    let src_x = out_pad_left + tx;
                    let src_y = out_pad_top + ty;
                    let src_i = src_y * out_tile_w + src_x;

                    let r = (out_data[src_i].clamp(0.0, 1.0) * 255.0) as u8;
                    let g = (out_data[out_area + src_i].clamp(0.0, 1.0) * 255.0) as u8;
                    let b = (out_data[out_area * 2 + src_i].clamp(0.0, 1.0) * 255.0) as u8;

                    out_img.put_pixel(
                        (in_x * SCALE + tx) as u32,
                        (in_y * SCALE + ty) as u32,
                        image::Rgb([r, g, b]),
                    );
                }
            }

            #[cfg(target_arch = "wasm32")]
            {
                let promise = js_sys::Promise::resolve(&wasm_bindgen::JsValue::NULL);
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            }
        }
    }

    Ok(out_img)
}

async fn run_inference<B: Backend>(
    model: &model::Model<B>,
    device: &B::Device,
    planar_pixels: Vec<f32>,
    tile_w: usize,
    tile_h: usize,
) -> Result<Vec<f32>> {
    let tensor_data = burn::tensor::TensorData::new(planar_pixels, vec![1, 3, tile_h, tile_w]);
    let input_tensor: Tensor<B, 4> = Tensor::from_data(tensor_data, device);
    let output_tensor = model.forward(input_tensor);
    let out_data = output_tensor
        .into_data_async()
        .await
        .map_err(|e| ProxyNexusError::Internal(format!("Inference failed: {}", e)))?
        .to_vec::<f32>()
        .map_err(|e| ProxyNexusError::Internal(format!("Data conversion failed: {}", e)))?;
    Ok(out_data)
}

fn finalize_upscale_img(out_img: image::RgbImage) -> Result<Vec<u8>> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 95);

    out_img
        .write_with_encoder(encoder)
        .map_err(|e| ProxyNexusError::Internal(e.to_string()))?;

    Ok(cursor.into_inner())
}

async fn fetch_model_weights() -> Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    {
        let response = gloo_net::http::Request::get("/realesr-general-x4v3.bpk")
            .send()
            .await
            .map_err(ProxyNexusError::Network)?;

        if !response.ok() {
            return Err(ProxyNexusError::Internal(format!(
                "Failed to fetch model weights for upscaler: HTTP {} {}",
                response.status(),
                response.status_text()
            )));
        }

        let bytes = response.binary().await.map_err(ProxyNexusError::Network)?;
        Ok(bytes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(include_bytes!(concat!(env!("OUT_DIR"), "/realesr-general-x4v3.bpk")).to_vec())
    }
}
