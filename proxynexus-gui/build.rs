use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&manifest_dir).parent().unwrap();

    let source_dir = root.join("proxynexus-core").join("assets");
    let dest_dir = Path::new(&manifest_dir).join("public").join("card_backs");

    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).expect("Failed to create card_backs directory");
    }

    println!("cargo:rerun-if-changed={}", source_dir.display());

    if let Ok(entries) = fs::read_dir(&source_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "png") {
                let filename = path.file_name().unwrap();
                let dest_path = dest_dir.join(filename);

                fs::copy(&path, &dest_path).unwrap_or_else(|e| {
                    panic!(
                        "Failed to copy {} to {}: {}",
                        path.display(),
                        dest_path.display(),
                        e
                    )
                });
            }
        }
    }

    if let Ok(bpk_path_str) = std::env::var("DEP_PROXYNEXUS_CORE_METADATA_MODEL_WEIGHTS_PATH") {
        let bpk_path = Path::new(&bpk_path_str);
        if bpk_path.exists() {
            let dest_path = Path::new(&manifest_dir)
                .join("public")
                .join("realesr-general-x4v3.bpk");
            fs::copy(bpk_path, &dest_path).expect("Failed to pull model weights from core");

            println!("cargo:rerun-if-changed={}", bpk_path_str);
        }
    }
}
