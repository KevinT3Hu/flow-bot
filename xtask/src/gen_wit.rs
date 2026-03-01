use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn run(project_root: &Path) -> Result<()> {
    println!("📝 Generating WIT Bindings");
    println!("=========================");
    println!();

    let wit_file = project_root.join("wit/onebot11.wit");
    let out_dir = project_root.join("flow-bot-plugin-sdk/src");

    if !wit_file.exists() {
        anyhow::bail!("WIT file not found: {}", wit_file.display());
    }

    if !out_dir.exists() {
        anyhow::bail!("Output directory not found: {}", out_dir.display());
    }

    println!("  Input:  {}", wit_file.display());
    println!("  Output: {}", out_dir.display());
    println!();

    let status = Command::new("wit-bindgen")
        .args([
            "rust",
            wit_file.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
            "--pub-export-macro",
            "--async",
            "all",
        ])
        .current_dir(project_root)
        .status()
        .with_context(|| "Failed to execute wit-bindgen. Is it installed?\nInstall with: cargo install wit-bindgen-cli")?;

    if !status.success() {
        anyhow::bail!("wit-bindgen failed with exit code: {}", status);
    }

    println!();
    println!("✅ WIT bindings generated successfully!");

    Ok(())
}
