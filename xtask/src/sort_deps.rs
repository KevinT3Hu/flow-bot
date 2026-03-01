use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Table};

const DEP_SECTIONS: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];

pub fn run(project_root: &Path, check: bool) -> Result<()> {
    println!("🔧 Sort Dependencies Task");
    println!("=========================");
    println!();

    let mut sorted_count = 0;
    let mut unsorted_files = Vec::new();

    for entry in walkdir::WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|e| !is_target_dir(e))
    {
        let entry = entry?;
        if entry.file_name() == "Cargo.toml" {
            let path = entry.path();
            let was_sorted = process_file(path, check)?;

            if was_sorted {
                if check {
                    unsorted_files.push(path.to_path_buf());
                } else {
                    println!("  Sorted: {}", path.display());
                    sorted_count += 1;
                }
            }
        }
    }

    println!();

    if check && !unsorted_files.is_empty() {
        eprintln!("❌ The following files have unsorted dependencies:");
        for path in &unsorted_files {
            eprintln!("  - {}", path.display());
        }
        anyhow::bail!("Dependencies are not sorted");
    }

    if check {
        println!("✅ All dependencies are sorted!");
    } else {
        println!("✅ Sorted dependencies in {} file(s)", sorted_count);
    }

    Ok(())
}

fn is_target_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        && (entry.file_name() == "target" || entry.file_name() == ".git")
}

fn process_file(path: &Path, check: bool) -> Result<bool> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    let mut was_modified = false;

    for section in DEP_SECTIONS {
        if let Some(Item::Table(table)) = doc.get_mut(section) {
            if sort_table(table) {
                was_modified = true;
            }
        }
    }

    if was_modified && !check {
        std::fs::write(path, doc.to_string())
            .with_context(|| format!("Failed to write {}", path.display()))?;
    }

    Ok(was_modified)
}

fn sort_table(table: &mut Table) -> bool {
    // Collect items and check if sorted
    let keys: Vec<_> = table.iter().map(|(k, _)| k.to_string()).collect();

    if keys.len() <= 1 {
        return false;
    }

    let mut sorted_keys = keys.clone();
    sorted_keys.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

    if keys == sorted_keys {
        return false;
    }

    // We need to sort - collect all items first
    let mut items: BTreeMap<String, (Item, Vec<(String, Item)>)> = BTreeMap::new();

    for key in &keys {
        // Get the item and its decoration
        if let Some(item) = table.remove(key) {
            items.insert(key.to_lowercase(), (item, Vec::new()));
        }
    }

    // Re-insert in sorted order
    for (orig_key, _) in sorted_keys.iter().map(|k| (k, k.to_lowercase())) {
        if let Some((item, _)) = items.remove(&orig_key.to_lowercase()) {
            table.insert(orig_key, item);
        }
    }

    true
}
