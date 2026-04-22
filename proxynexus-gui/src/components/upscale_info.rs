use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct UpscaleInfoProps {
    pub on_close: EventHandler<()>,
    pub pos: (f64, f64, f64),
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Browser {
    Chrome,
    Firefox,
    Safari,
    Mobile,
    Other,
}

fn get_browser() -> Browser {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(ua) = window.navigator().user_agent() {
                let ua_lower = ua.to_lowercase();
                if ua_lower.contains("mobi")
                    || ua_lower.contains("android")
                    || ua_lower.contains("iphone")
                    || ua_lower.contains("ipad")
                {
                    return Browser::Mobile;
                }
                if ua_lower.contains("chrome")
                    || ua_lower.contains("chromium")
                    || ua_lower.contains("edg")
                {
                    return Browser::Chrome;
                }
                if ua_lower.contains("firefox") {
                    return Browser::Firefox;
                }
                if ua_lower.contains("safari") {
                    return Browser::Safari;
                }
            }
        }
    }
    Browser::Other
}

#[component]
pub fn UpscaleInfo(props: UpscaleInfoProps) -> Element {
    let (x, y, w) = props.pos;
    let is_gpu_available = proxynexus_core::is_gpu_available();
    let browser = get_browser();
    let is_web = cfg!(target_arch = "wasm32");

    rsx! {
        div {
            class: "fixed inset-0 z-[2000]",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "absolute max-md:!fixed max-md:!top-1/2 max-md:!left-1/2 max-md:![transform:translate(-50%,-50%)] bg-white p-6 rounded-lg shadow-2xl border border-gray-200 w-[90vw] md:w-96 select-text",
                style: "top: {y - 12.0}px; left: {x + w / 2.0}px; transform: translate(-50%, -100%);",
                onclick: move |evt| evt.stop_propagation(),

                button {
                    class: "absolute top-4 right-4 text-gray-400 hover:text-gray-600 focus:outline-none transition-colors",
                    onclick: move |_| props.on_close.call(()),
                    svg {
                        class: "w-5 h-5",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M6 18L18 6M6 6l12 12" }
                    }
                }

                div { class: "flex flex-col gap-4 text-sm mt-2",
                    div {
                        h4 { class: "font-semibold mb-1", "Upscaling" }
                        p { class: "text-gray-600 leading-relaxed",
                            "Uses a Real-ESRGAN-based model to upscale card images. It doesn't just resize, it reconstructs fine detail, improving clarity for text, edges, and artwork. "
                            if is_web && is_gpu_available {
                                "Runs locally in your browser using your GPU."
                            }
                        }
                    }

                    if is_web {
                        div { class: "flex items-center gap-2",
                            if is_gpu_available {
                                span { class: "w-2.5 h-2.5 rounded-full bg-green-500 shadow-sm" }
                                span { class: "font-medium text-gray-700", "WebGPU Available" }
                            } else {
                                span { class: "w-2.5 h-2.5 rounded-full bg-red-500 shadow-sm" }
                                span { class: "font-medium text-gray-700", "WebGPU Not Available" }
                            }
                        }
                    }

                    if is_web && !is_gpu_available {
                        div { class: "flex flex-col gap-1 border-t border-gray-100",
                            p { class: "text-gray-700 font-medium",
                                "Requires WebGPU to be enabled in your browser."
                            }

                            div { class: "bg-gray-50 p-3 rounded border border-gray-200 text-gray-600 space-y-2",
                                match browser {
                                    Browser::Chrome => rsx! {
                                        p { "1. Paste or type " span { class: "font-mono", "chrome://flags/#enable-unsafe-webgpu" } " in the address bar and press Enter." }
                                        p { "2. Set " span { class: "font-bold text-gray-700", "Unsafe WebGPU Support" } " to " span { class: "font-bold text-gray-700", "Enabled" } "." }
                                        p { "3. Reload " span { class: "font-mono", "https://proxynexus.net" } "." }
                                    },
                                    Browser::Firefox => rsx! {
                                        p { "1. Paste or type " span { class: "font-mono", "about:config" } " in the address bar and press Enter." }
                                        p { "2. Click \"Accept the Risk and Continue\"." }
                                        p { "3. Search for " span { class: "font-mono", "dom.webgpu.enabled" } " and set to " span { class: "font-bold text-gray-700", "true" } "." }
                                        p { "4. Reload " span { class: "font-mono", "https://proxynexus.net" } "." }
                                    },
                                    Browser::Safari => rsx! {
                                        p { "1. Safari → Settings → Advanced → " span { class: "font-bold text-gray-700", "\"Show features for web developers\"" } "." }
                                        p { "2. Develop → Experimental Features → " span { class: "font-bold text-gray-700", "WebGPU" } "." }
                                    },
                                    Browser::Mobile => rsx! {
                                        p { class: "italic", "Upscaling is not supported on mobile browsers. Please use a desktop browser to enable upscaling." }
                                    },
                                    Browser::Other => rsx! {
                                        p { "Please check your browser settings or experimental flags to enable WebGPU support." }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
