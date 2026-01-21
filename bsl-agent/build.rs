use std::path::PathBuf;

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .expect("bsl-agent crate must be in repo root");

    let site_dir = repo_root.join("target").join("site");
    let index_path = site_dir.join("index.html");

    println!("cargo:rerun-if-changed={}", index_path.display());
    println!("cargo:rerun-if-changed={}", site_dir.display());

    if !index_path.is_file() {
        eprintln!(
            "bsl-agent embedded UI assets not found: {}\n\
             Build frontend first, e.g.:\n\
             \n\
             (cd frontend && NO_COLOR=true trunk build --release)\n\
             \n\
             Expected output directory:\n\
             {}\n",
            index_path.display(),
            site_dir.display()
        );
        std::process::exit(1);
    }
}
