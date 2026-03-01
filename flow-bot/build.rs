use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Check if webui feature is enabled
    let webui_enabled = env::var("CARGO_FEATURE_WEBUI").is_ok();

    if !webui_enabled {
        println!("cargo::warning=Web UI feature not enabled. Build with --features webui to include the web interface.");
        return;
    }

    // Only rebuild if building in release mode or if FORCE_WEB_BUILD is set
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let force_build = env::var("FORCE_WEB_BUILD").is_ok();

    // Skip web build in debug mode unless forced (to speed up development)
    if profile == "debug" && !force_build {
        println!("cargo::warning=Skipping web UI build in debug mode. Set FORCE_WEB_BUILD=1 to force build.");
        return;
    }

    // Paths are relative to the package directory (where this Cargo.toml is)
    let ui_dir = Path::new("src/web/ui");
    let static_dir = Path::new("src/web/static");

    // Check if npm is available
    if Command::new("npm").arg("--version").output().is_err() {
        println!("cargo::warning=npm not found, skipping web UI build");
        println!("cargo::warning=To build the web UI, install Node.js and npm");
        return;
    }

    // Check if node_modules exists, if not run npm install
    if !ui_dir.join("node_modules").exists() {
        println!("cargo::rerun-if-changed={}", ui_dir.join("package.json").display());

        let output = Command::new("npm")
            .arg("install")
            .current_dir(ui_dir)
            .output()
            .expect("Failed to run npm install");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("npm install failed: {}", stderr);
        }

        println!("cargo::warning=Web UI dependencies installed successfully");
    }

    // Run npm run build
    println!("cargo::rerun-if-changed={}", ui_dir.join("src").display());
    println!("cargo::rerun-if-changed={}", ui_dir.join("index.html").display());

    let output = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(ui_dir)
        .output()
        .expect("Failed to run npm run build");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("npm run build failed: {}", stderr);
    }

    println!("cargo::warning=Web UI built successfully");

    // Tell cargo to watch the static directory for changes
    println!("cargo::rerun-if-changed={}", static_dir.display());
}
