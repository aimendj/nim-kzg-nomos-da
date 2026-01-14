use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    
    // Tell cargo to rerun this build script if source files change
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    
    let config_path = PathBuf::from(&crate_dir).join("cbindgen.toml");
    let header_path = PathBuf::from(&crate_dir).join("nomos_da_ffi.h");
    
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .with_config(cbindgen::Config::from_file(config_path).unwrap())
        .generate()
        .expect("Unable to generate C bindings")
        .write_to_file(header_path);
}
