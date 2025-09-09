//! Build script for bsl-gradual-types
//! Skip native tree-sitter stubs on wasm32 targets

fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_arch == "wasm32" {
        println!("cargo:rerun-if-changed=build.rs");
        return;
    }

    // Attempt to link local tree-sitter-bsl if present
    let tree_sitter_bsl_path = "../tree-sitter-bsl";
    let parser_c_path = format!("{}/src/parser.c", tree_sitter_bsl_path);

    if std::path::Path::new(&parser_c_path).exists() {
        cc::Build::new()
            .std("c11")
            .include(format!("{}/src", tree_sitter_bsl_path))
            .file(&parser_c_path)
            .compile("tree_sitter_bsl");
        println!("cargo:rustc-link-lib=tree_sitter_bsl");
        return;
    }

    println!("cargo:rerun-if-changed=build.rs");
}
