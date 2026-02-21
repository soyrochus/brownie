mod app;
mod copilot;
mod event;
mod session;
mod theme;
mod ui;

use app::BrownieApp;
use copilot::CopilotClient;
use eframe::egui;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::mpsc;

fn should_skip_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git") | Some("target")
    )
}

fn to_workspace_relative(path: &Path, workspace: &Path) -> String {
    path.strip_prefix(workspace)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn detect_instruction_files(workspace: &Path) -> Vec<String> {
    let mut discovered = BTreeSet::new();
    let known_files = [
        workspace.join(".github/copilot-instructions.md"),
        workspace.join("AGENTS.md"),
    ];

    for known_file in &known_files {
        if known_file.exists() {
            discovered.insert(to_workspace_relative(known_file, workspace));
        }
    }

    let mut stack = vec![workspace.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if should_skip_dir(&path) {
                    continue;
                }
                stack.push(path);
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if file_name.ends_with(".instructions.md") {
                discovered.insert(to_workspace_relative(&path, workspace));
            }
        }
    }

    discovered.into_iter().collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = std::env::current_dir()?;
    let instruction_files = detect_instruction_files(&workspace);
    let (tx, rx) = mpsc::channel();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("brownie-runtime")
        .build()?;

    let copilot = runtime.block_on(async { CopilotClient::new(workspace.clone(), tx.clone()) })?;
    copilot.start();

    let app = BrownieApp::new(rx, copilot, workspace, instruction_files);
    let _runtime = runtime;

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([1024.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Brownie",
        native_options,
        Box::new(move |_creation_context| Ok(Box::new(app))),
    )?;

    Ok(())
}
