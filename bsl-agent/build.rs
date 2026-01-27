use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn run_git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8(out.stdout).ok()?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .expect("bsl-agent crate must be in repo root");

    // Build identity (best-effort).
    let build_unix_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    println!(
        "cargo:rustc-env=BSL_AGENT_BUILD_UNIX_SECS={}",
        build_unix_secs
    );

    if let Ok(profile) = std::env::var("PROFILE") {
        if !profile.trim().is_empty() {
            println!("cargo:rustc-env=BSL_AGENT_PROFILE={}", profile.trim());
        }
    }
    if let Ok(target) = std::env::var("TARGET") {
        if !target.trim().is_empty() {
            println!("cargo:rustc-env=BSL_AGENT_TARGET={}", target.trim());
        }
    }

    let git_sha = run_git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=BSL_AGENT_GIT_SHA={}", git_sha);

    if let Some(describe) = run_git(&["describe", "--always", "--dirty", "--tags"]) {
        println!("cargo:rustc-env=BSL_AGENT_GIT_DESCRIBE={}", describe);
    }

    let git_dir = repo_root.join(".git");
    // Rebuild when git metadata changes (best-effort; may be absent in some environments).
    println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
    println!("cargo:rerun-if-changed={}", git_dir.join("refs").display());
    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join("packed-refs").display()
    );

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
