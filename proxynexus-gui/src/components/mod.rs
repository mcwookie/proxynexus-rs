pub mod about_modal;
pub mod card_list_input;
pub mod export_controls;
pub mod preview_grid;
pub mod print_layout_info;
pub mod source_selector;
pub mod upscale_info;
pub mod variant_selector;

pub(crate) fn build_image_url(image_key: &str) -> String {
    #[cfg(feature = "desktop")]
    {
        format!("proxynexus://collections/{}", image_key)
    }

    #[cfg(feature = "web")]
    {
        // Override at build time with PROXYNEXUS_COLLECTIONS_URL to point at
        // a self-hosted bucket (e.g. MinIO) instead of the upstream
        // maintainer's Cloudflare R2 bucket. Falls back to the original
        // default so upstream behavior is unchanged if unset.
        const BASE_URL: &str = match option_env!("PROXYNEXUS_COLLECTIONS_URL") {
            Some(url) => url,
            None => "https://collections.proxynexus.net",
        };
        format!("{}/{}", BASE_URL, image_key)
    }
}
