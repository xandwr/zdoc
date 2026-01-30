use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use clap::{Parser, Subcommand};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "zdoc",
    version,
    about = "A lean, terminal-first Rust documentation parser"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fuzzy search query within a crate or globally
    Search {
        /// The search term
        query: String,
        /// The crate to search within (optional)
        crate_name: Option<String>,
        /// Limit results
        #[arg(short, long, default_value_t = 5)]
        results: usize,
    },
    /// Diff public API between versions
    Diff {
        crate_name: String,
        ver1: String,
        ver2: String,
    },
    /// List available features
    Features { crate_name: String },
}

fn search_docs(
    metadata: &cargo_metadata::Metadata,
    crate_name: Option<&str>,
    query: &str,
    limit: usize,
) -> Result<()> {
    // Step 1: Run cargo doc with JSON output format (requires nightly or RUSTC_BOOTSTRAP)
    println!("Generating JSON documentation...");

    // Try to generate docs for dependencies and this crate
    let status = Command::new("cargo")
        .arg("doc")
        .env("RUSTDOCFLAGS", "-Z unstable-options --output-format json")
        .env("RUSTC_BOOTSTRAP", "1") // Enable unstable features on stable
        .status()
        .context("Failed to run `cargo doc`. Make sure you have Rust installed.")?;

    if !status.success() {
        println!("Warning: cargo doc returned non-zero status, but continuing...");
    }

    // Step 2: Find the generated JSON file(s)
    let target_dir = &metadata.target_directory;
    let doc_dir = PathBuf::from(target_dir).join("doc");

    // Get the crate(s) to search
    let crates_to_search: Vec<String> = if let Some(name) = crate_name {
        vec![name.to_string()]
    } else {
        // Search all workspace crates
        metadata
            .workspace_packages()
            .iter()
            .map(|p| p.name.to_string())
            .collect()
    };

    // Step 3 & 4: Load JSON files and fuzzy match
    let mut all_results = Vec::new();

    for crate_name in &crates_to_search {
        let json_path = doc_dir.join(format!("{}.json", crate_name));

        if !json_path.exists() {
            continue; // Skip if JSON doesn't exist for this crate
        }

        let json_content = fs::read_to_string(&json_path)
            .with_context(|| format!("Failed to read {}", json_path.display()))?;

        let json_data: Value = serde_json::from_str(&json_content)
            .with_context(|| format!("Failed to parse JSON from {}", json_path.display()))?;

        let matches = fuzzy_search_json(&json_data, crate_name, query)?;
        all_results.extend(matches);
    }

    // Sort by score and limit
    all_results.sort_by(|a, b| b.score.cmp(&a.score));
    all_results.truncate(limit);

    // Display results
    if all_results.is_empty() {
        println!("No matches found for '{}'", query);
    } else {
        println!("\nSearch results for '{}':\n", query);
        for (i, result) in all_results.iter().enumerate() {
            println!("{}. {} ({})", i + 1, result.name, result.item_type);
            println!("   Crate: {}", result.crate_name);
            if let Some(path) = &result.path {
                println!("   Path: {}", path);
            }
            if let Some(desc) = &result.description {
                let desc_preview: String = desc.chars().take(100).collect();
                println!(
                    "   {}{}",
                    desc_preview,
                    if desc.len() > 100 { "..." } else { "" }
                );
            }
            println!();
        }
    }

    Ok(())
}

#[derive(Debug)]
struct SearchResult {
    name: String,
    crate_name: String,
    item_type: String,
    path: Option<String>,
    description: Option<String>,
    score: i64,
}

fn fuzzy_search_json(
    json_data: &Value,
    crate_name: &str,
    query: &str,
) -> Result<Vec<SearchResult>> {
    let matcher = SkimMatcherV2::default();
    let mut results = Vec::new();

    // Get the index object from the JSON
    let index = json_data
        .get("index")
        .and_then(|v| v.as_object())
        .context("Missing or invalid 'index' field in JSON")?;

    // Search through all items in the index
    for (_id, item) in index {
        // Get the item name
        let name = match item.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => continue, // Skip unnamed items
        };

        // Fuzzy match against the query
        if let Some(score) = matcher.fuzzy_match(name, query) {
            // Get the item type from the "inner" field
            let item_type = item
                .get("inner")
                .and_then(|inner| inner.as_object())
                .and_then(|obj| obj.keys().next().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());

            // Extract documentation if available
            let description = item
                .get("docs")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            results.push(SearchResult {
                name: name.to_string(),
                crate_name: crate_name.to_string(),
                item_type,
                path: None, // We'll skip path building for simplicity
                description,
                score,
            });
        }
    }

    Ok(results)
}

fn main() -> Result<()> {
    // 1. Immediate constraint check
    if !Path::new("Cargo.toml").exists() {
        anyhow::bail!("Error: No `Cargo.toml` found. `zdoc` must be run within a Rust project.");
    }

    let cli = Cli::parse();

    // 2. Fetch project metadata (this is fast after the first run)
    let metadata = MetadataCommand::new()
        .exec()
        .context("Failed to parse cargo metadata")?;

    match &cli.command {
        Commands::Search {
            query,
            crate_name,
            results,
        } => {
            search_docs(&metadata, crate_name.as_deref(), query, *results)?;
        }

        Commands::Features { crate_name } => {
            // Find the package in the metadata
            let package = metadata
                .packages
                .iter()
                .find(|p| p.name == *crate_name)
                .with_context(|| format!("Crate '{}' not found in dependencies", crate_name))?;

            println!("Features for {} (v{}):", package.name, package.version);

            if package.features.is_empty() {
                println!("  (No features defined)");
            } else {
                for (feature, deps) in &package.features {
                    let dep_list = if deps.is_empty() {
                        "".to_string()
                    } else {
                        format!(" -> {}", deps.join(", "))
                    };
                    println!("  [ ] {} {}", feature, dep_list);
                }
            }
        }

        _ => todo!("Implementing logic for other commands..."),
    }

    Ok(())
}
