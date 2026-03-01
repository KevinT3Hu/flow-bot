use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

const CRATES: &[(&str, &str)] = &[
    ("flow-bot-onebot11", "flow-bot-onebot11"),
    ("flow-bot-extractor", "flow-bot-extractor"),
    ("flow-bot-plugin-sdk", "flow-bot-plugin-sdk"),
    ("flow-bot", "flow-bot"),
];

pub fn run(project_root: &Path, dry_run: bool, allow_dirty: bool, no_verify: bool) -> Result<()> {
    println!("🔧 Flow-Bot Publish Task");
    println!("========================");
    println!();

    // Check if cargo is installed
    if !command_exists("cargo") {
        bail!("cargo is not installed");
    }

    if dry_run {
        println!("🏃 Running in dry-run mode (no actual publish)");
        println!();
    }

    println!("📍 Project root: {}", project_root.display());
    println!();

    println!("🚀 Starting publish process...");
    println!();

    for (crate_path, crate_name) in CRATES {
        publish_crate(
            project_root,
            crate_path,
            crate_name,
            dry_run,
            allow_dirty,
            no_verify,
        )?;
    }

    println!("✅ All packages published successfully!");
    Ok(())
}

fn publish_crate(
    project_root: &Path,
    crate_path: &str,
    crate_name: &str,
    dry_run: bool,
    allow_dirty: bool,
    no_verify: bool,
) -> Result<()> {
    let full_path = project_root.join(crate_path);

    println!("📦 Publishing {}...", crate_name);

    // Get version from Cargo.toml
    let cargo_toml = full_path.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml)
        .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

    let version = extract_version(&content)
        .with_context(|| format!("Failed to extract version from {}", crate_name))?;

    // Check if already published
    if !dry_run {
        let output = Command::new("cargo")
            .args(["search", crate_name, "--limit", "1"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let search_result = format!("{} = \"{}\"", crate_name, version);

        if stdout.contains(&search_result) {
            println!(
                "  ⚠ {} v{} is already published, skipping",
                crate_name, version
            );
            println!();
            return Ok(());
        }
    }

    // Build arguments for cargo publish
    let mut args = vec!["publish"];

    if dry_run {
        args.push("--dry-run");
    }
    if allow_dirty {
        args.push("--allow-dirty");
    }
    if no_verify {
        args.push("--no-verify");
    }

    // Build and verify (unless dry-run which already does this)
    if !dry_run {
        println!("  Building and verifying...");
        let status = Command::new("cargo")
            .current_dir(&full_path)
            .args(["build", "--release"])
            .status()?;

        if !status.success() {
            bail!("Failed to build {}", crate_name);
        }
    }

    println!("  Publishing to crates.io...");
    let status = Command::new("cargo")
        .current_dir(&full_path)
        .args(&args)
        .status()?;

    if !status.success() {
        bail!("Failed to publish {}", crate_name);
    }

    if dry_run {
        println!("  ✓ {} v{} dry-run passed", crate_name, version);
    } else {
        println!("  ✓ Published {} v{}", crate_name, version);
    }
    println!();

    Ok(())
}

fn extract_version(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("version") {
            // Extract version from: version = "x.y.z"
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    return Some(line[start + 1..start + 1 + end].to_string());
                }
            }
        }
    }
    None
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
