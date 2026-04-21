fn main() {
    #[cfg(feature = "upscaling")]
    {
        use burn_onnx::ModelGen;
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let onnx_file = "assets/realesr-general-x4v3.onnx";
        println!("cargo:rerun-if-changed={}", onnx_file);

        ModelGen::new()
            .input(onnx_file)
            .out_dir(&out_dir)
            .run_from_script();

        let bpk_path = std::path::Path::new(&out_dir).join("realesr-general-x4v3.bpk");
        if bpk_path.exists() {
            println!("cargo:metadata_bpk_path={}", bpk_path.display());
        }
    }
}
