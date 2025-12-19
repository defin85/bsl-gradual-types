use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use bsl_backend::data::loaders::{
    compare_module_parsing_from_file_with_progress_mode, single_pass_module_stats_from_file_with_progress_mode,
    SinglePassMode,
};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut module: Option<PathBuf> = None;
    let mut mode = SinglePassMode::Full;
    let mut skip_ast = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--file" => {
                let Some(path) = args.next() else {
                    return Err(anyhow!("--file requires a path"));
                };
                module = Some(PathBuf::from(path));
            }
            "--single-pass-lite" => mode = SinglePassMode::Lite,
            "--single-pass-full" => mode = SinglePassMode::Full,
            "--skip-ast" => skip_ast = true,
            _ => {
                if module.is_none() {
                    module = Some(PathBuf::from(arg));
                }
            }
        }
    }

    let mut module = module.unwrap_or_else(|| PathBuf::from("examples/conf_big"));

    if module.is_dir() {
        eprintln!("Scanning modules under {} ...", module.display());
        module = find_largest_module(&module)?;
    }

    eprintln!(
        "Comparing module: {} (size {} bytes)",
        module.display(),
        module.metadata().map(|m| m.len()).unwrap_or(0)
    );
    eprintln!("Single-pass mode: {:?}", mode);

    let started = Instant::now();
    let mut last_step = started;
    let ast_running = Arc::new(AtomicBool::new(false));
    let ast_started = Arc::new(Mutex::new(None::<Instant>));
    let mut ast_heartbeat: Option<thread::JoinHandle<()>> = None;

    if skip_ast {
        let stats =
            single_pass_module_stats_from_file_with_progress_mode(&module, mode, |stage| {
                let now = Instant::now();
                eprintln!(
                    "[{}] {} (+{})",
                    format_duration(now - started),
                    stage,
                    format_duration(now - last_step)
                );
                last_step = now;
            })?;
        eprintln!("Done in {}", format_duration(started.elapsed()));
        println!(
            "Single-pass: decls={}, exports={}, call_sites={}",
            stats.decls, stats.export_decls, stats.call_sites
        );
        return Ok(());
    }

    let comparison = compare_module_parsing_from_file_with_progress_mode(&module, mode, |stage| {
        let now = Instant::now();
        eprintln!(
            "[{}] {} (+{})",
            format_duration(now - started),
            stage,
            format_duration(now - last_step)
        );
        last_step = now;

        if stage.starts_with("AST конвертация") {
            if !ast_running.swap(true, Ordering::SeqCst) {
                if let Ok(mut guard) = ast_started.lock() {
                    *guard = Some(Instant::now());
                }
                let ast_running = Arc::clone(&ast_running);
                let ast_started = Arc::clone(&ast_started);
                ast_heartbeat = Some(thread::spawn(move || {
                    let sleep = Duration::from_secs(10);
                    while ast_running.load(Ordering::SeqCst) {
                        thread::sleep(sleep);
                        if !ast_running.load(Ordering::SeqCst) {
                            break;
                        }
                        let elapsed = ast_started
                            .lock()
                            .ok()
                            .and_then(|t| t.map(|t| t.elapsed()))
                            .unwrap_or_else(|| Duration::from_secs(0));
                        eprintln!(
                            "[{}] AST конвертация: в работе (+{})",
                            format_duration(Instant::now() - started),
                            format_duration(elapsed)
                        );
                    }
                }));
            }
        } else if ast_running.swap(false, Ordering::SeqCst) {
            if let Some(handle) = ast_heartbeat.take() {
                let _ = handle.join();
            }
        }
    })?;

    if ast_running.swap(false, Ordering::SeqCst) {
        if let Some(handle) = ast_heartbeat.take() {
            let _ = handle.join();
        }
    }
    eprintln!("Done in {}", format_duration(started.elapsed()));

    println!("Module: {}", comparison.module_path.display());
    println!(
        "Single-pass: decls={}, exports={}, call_sites={}",
        comparison.single_pass.decls,
        comparison.single_pass.export_decls,
        comparison.single_pass.call_sites
    );
    println!(
        "AST:         decls={}, exports={}, call_sites={}",
        comparison.ast.decls,
        comparison.ast.export_decls,
        comparison.ast.call_sites
    );

    if comparison.missing_decls.is_empty()
        && comparison.extra_decls.is_empty()
        && comparison.callsite_mismatches.is_empty()
    {
        println!("Comparison: OK (no mismatches)");
        return Ok(());
    }

    if !comparison.missing_decls.is_empty() {
        println!("Missing decls in single-pass (top 10):");
        for item in comparison.missing_decls.iter().take(10) {
            println!("  {}", item);
        }
    }

    if !comparison.extra_decls.is_empty() {
        println!("Extra decls in single-pass (top 10):");
        for item in comparison.extra_decls.iter().take(10) {
            println!("  {}", item);
        }
    }

    if !comparison.callsite_mismatches.is_empty() {
        println!("Callsite mismatches (top 10):");
        for item in comparison.callsite_mismatches.iter().take(10) {
            println!("  {}", item);
        }
    }

    Ok(())
}

fn find_largest_module(root: &Path) -> Result<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut best_path: Option<PathBuf> = None;
    let mut best_size: u64 = 0;

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };

            let is_module = matches!(
                file_name,
                "Module.bsl" | "ObjectModule.bsl" | "ManagerModule.bsl" | "RecordSetModule.bsl"
            );
            if !is_module {
                continue;
            }

            let size = match path.metadata() {
                Ok(m) => m.len(),
                Err(_) => continue,
            };

            if size > best_size {
                best_size = size;
                best_path = Some(path);
                eprintln!(
                    "New largest module: {} ({} bytes)",
                    best_path.as_ref().unwrap().display(),
                    best_size
                );
            }
        }
    }

    best_path.ok_or_else(|| anyhow!("Module files not found under {}", root.display()))
}

fn format_duration(duration: Duration) -> String {
    let mut secs = duration.as_secs();
    let hours = secs / 3600;
    secs %= 3600;
    let minutes = secs / 60;
    secs %= 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    }
}
