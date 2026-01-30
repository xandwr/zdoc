use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use clap::{Parser, Subcommand};
use colored::Colorize;
use flate2::read::GzDecoder;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
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

// Data structures for diff functionality
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ApiItem {
    name: String,
    item_type: String,
    path: Vec<String>,
    signature: String, // Serialized representation of the signature
}

impl ApiItem {
    fn full_path(&self) -> String {
        if self.path.is_empty() {
            self.name.clone()
        } else {
            format!("{}::{}", self.path.join("::"), self.name)
        }
    }

    fn display_string(&self) -> String {
        format!("{} {}", self.item_type, self.full_path())
    }
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

// Fetch rustdoc JSON from docs.rs
async fn fetch_docs_json(crate_name: &str, version: &str) -> Result<Value> {
    // docs.rs serves JSON files compressed with gzip
    let url = format!("https://docs.rs/crate/{}/{}/json.gz", crate_name, version);

    println!("Fetching documentation for {} v{}...", crate_name, version);

    let response = reqwest::get(&url)
        .await
        .context(format!("Failed to fetch docs from {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch docs for {} v{}: HTTP {}. Make sure the version exists on docs.rs and has JSON docs available (added May 2025).",
            crate_name,
            version,
            response.status()
        );
    }

    let compressed_bytes = response
        .bytes()
        .await
        .context("Failed to read response body")?;

    // Explicitly decompress the gzip data
    let mut decoder = GzDecoder::new(&compressed_bytes[..]);
    let mut json_text = String::new();
    decoder
        .read_to_string(&mut json_text)
        .context("Failed to decompress gzip data")?;

    let json_data: Value =
        serde_json::from_str(&json_text).context("Failed to parse JSON response")?;

    Ok(json_data)
}

// Extract API items from rustdoc JSON with signature details
fn extract_api_items(json_data: &Value) -> Result<Vec<ApiItem>> {
    let mut items = Vec::new();

    let index = json_data
        .get("index")
        .and_then(|v| v.as_object())
        .context("Missing or invalid 'index' field in JSON")?;

    // Build a map of item IDs to their paths
    let mut id_to_path: HashMap<String, Vec<String>> = HashMap::new();

    // First pass: collect all items and build path information
    for (id, item) in index {
        if item.get("name").and_then(|v| v.as_str()).is_some() {
            // Try to get the path from "path" field
            let path = item
                .get("path")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            id_to_path.insert(id.clone(), path);
        }
    }

    // Second pass: extract items with their signatures
    for (id, item) in index {
        let name = match item.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let inner = match item.get("inner").and_then(|v| v.as_object()) {
            Some(i) => i,
            None => continue,
        };

        let item_type = inner.keys().next().map(String::from).unwrap_or_default();

        // Skip certain internal items
        if item_type == "Import" || item_type == "ProcMacro" {
            continue;
        }

        let path = id_to_path.get(id).cloned().unwrap_or_default();

        // Extract signature based on item type
        let signature = extract_signature(&item_type, inner.get(&item_type));

        items.push(ApiItem {
            name,
            item_type,
            path,
            signature,
        });
    }

    Ok(items)
}

// Extract signature details for different item types
fn extract_signature(item_type: &str, inner_data: Option<&Value>) -> String {
    let inner = match inner_data {
        Some(d) => d,
        None => return String::new(),
    };

    match item_type {
        "Function" | "Method" => {
            // Extract function signature: parameters and return type
            let mut sig_parts = Vec::new();

            // Get parameters
            if let Some(decl) = inner.get("decl") {
                if let Some(inputs) = decl.get("inputs").and_then(|v| v.as_array()) {
                    let params: Vec<String> = inputs
                        .iter()
                        .filter_map(|input| {
                            let name = input.get(0).and_then(|v| v.as_str())?;
                            let type_str = format_type(input.get(1)?);
                            Some(format!("{}: {}", name, type_str))
                        })
                        .collect();
                    sig_parts.push(format!("({})", params.join(", ")));
                }

                // Get return type
                if let Some(output) = decl.get("output") {
                    if !output.is_null() {
                        let ret_type = format_type(output);
                        if ret_type != "()" {
                            sig_parts.push(format!("-> {}", ret_type));
                        }
                    }
                }
            }

            sig_parts.join(" ")
        }

        "Struct" => {
            // Extract struct fields
            if let Some(kind) = inner.get("kind") {
                if let Some(kind_str) = kind.as_str() {
                    match kind_str {
                        "plain" => {
                            if let Some(fields) = inner.get("fields").and_then(|v| v.as_array()) {
                                let field_sigs: Vec<String> = fields
                                    .iter()
                                    .filter_map(|field_id| {
                                        // This is a simplified version; proper implementation would
                                        // look up field details from index
                                        field_id.as_str().map(String::from)
                                    })
                                    .collect();
                                return format!("{{ {} fields }}", field_sigs.len());
                            }
                        }
                        "tuple" => {
                            if let Some(fields) = inner.get("fields").and_then(|v| v.as_array()) {
                                return format!("({} fields)", fields.len());
                            }
                        }
                        "unit" => return "".to_string(),
                        _ => {}
                    }
                }
            }
            String::new()
        }

        "Enum" => {
            // Extract enum variants
            if let Some(variants) = inner.get("variants").and_then(|v| v.as_array()) {
                return format!("{{ {} variants }}", variants.len());
            }
            String::new()
        }

        "Trait" => {
            // Extract trait items (methods, associated types)
            if let Some(items) = inner.get("items").and_then(|v| v.as_array()) {
                return format!("{{ {} items }}", items.len());
            }
            String::new()
        }

        _ => String::new(),
    }
}

// Helper to format type information from JSON
fn format_type(type_data: &Value) -> String {
    // This is a simplified type formatter
    // Real rustdoc JSON has complex nested type structures
    if let Some(resolved_path) = type_data.get("resolved_path") {
        if let Some(name) = resolved_path.get("name").and_then(|v| v.as_str()) {
            return name.to_string();
        }
    }

    if let Some(primitive) = type_data.get("primitive").and_then(|v| v.as_str()) {
        return primitive.to_string();
    }

    if let Some(borrowed_ref) = type_data.get("borrowed_ref") {
        let mutable = borrowed_ref
            .get("mutable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let inner_type = borrowed_ref
            .get("type")
            .map(format_type)
            .unwrap_or_else(|| "?".to_string());
        return if mutable {
            format!("&mut {}", inner_type)
        } else {
            format!("&{}", inner_type)
        };
    }

    // Fallback for complex types
    "...".to_string()
}

// Compare two sets of API items and categorize changes
fn compare_api_items(
    old_items: Vec<ApiItem>,
    new_items: Vec<ApiItem>,
) -> (Vec<ApiItem>, Vec<ApiItem>, Vec<(ApiItem, ApiItem)>) {
    let old_set: HashMap<String, ApiItem> = old_items
        .into_iter()
        .map(|item| (format!("{}::{}", item.full_path(), item.item_type), item))
        .collect();

    let new_set: HashMap<String, ApiItem> = new_items
        .into_iter()
        .map(|item| (format!("{}::{}", item.full_path(), item.item_type), item))
        .collect();

    let old_keys: HashSet<_> = old_set.keys().cloned().collect();
    let new_keys: HashSet<_> = new_set.keys().cloned().collect();

    // Items only in new version (added)
    let added: Vec<ApiItem> = new_keys
        .difference(&old_keys)
        .filter_map(|key| new_set.get(key).cloned())
        .collect();

    // Items only in old version (removed)
    let removed: Vec<ApiItem> = old_keys
        .difference(&new_keys)
        .filter_map(|key| old_set.get(key).cloned())
        .collect();

    // Items in both but with different signatures (modified)
    let modified: Vec<(ApiItem, ApiItem)> = old_keys
        .intersection(&new_keys)
        .filter_map(|key| {
            let old_item = old_set.get(key)?;
            let new_item = new_set.get(key)?;
            if old_item.signature != new_item.signature {
                Some((old_item.clone(), new_item.clone()))
            } else {
                None
            }
        })
        .collect();

    (added, removed, modified)
}

// Display diff results with git-style colored output
fn display_diff(
    crate_name: &str,
    ver1: &str,
    ver2: &str,
    mut added: Vec<ApiItem>,
    mut removed: Vec<ApiItem>,
    mut modified: Vec<(ApiItem, ApiItem)>,
) {
    println!(
        "\nAPI diff for {} ({}...{}):\n",
        crate_name.bold(),
        ver1,
        ver2
    );

    let added_count = added.len();
    let removed_count = removed.len();
    let modified_count = modified.len();

    let total_changes = added_count + removed_count + modified_count;
    if total_changes == 0 {
        println!("{}", "No API changes detected.".dimmed());
        return;
    }

    // Display removed items (red with -)
    if !removed.is_empty() {
        println!("{}", format!("Removed ({}):", removed_count).red().bold());
        removed.sort_by(|a, b| a.full_path().cmp(&b.full_path()));
        for item in removed {
            let display = format!("- {} {}", item.display_string(), item.signature);
            println!("  {}", display.red());
        }
        println!();
    }

    // Display added items (green with +)
    if !added.is_empty() {
        println!("{}", format!("Added ({}):", added_count).green().bold());
        added.sort_by(|a, b| a.full_path().cmp(&b.full_path()));
        for item in added {
            let display = format!("+ {} {}", item.display_string(), item.signature);
            println!("  {}", display.green());
        }
        println!();
    }

    // Display modified items (yellow with ~)
    if !modified.is_empty() {
        println!(
            "{}",
            format!("Modified ({}):", modified_count).yellow().bold()
        );
        modified.sort_by(|a, b| a.0.full_path().cmp(&b.0.full_path()));
        for (old_item, new_item) in modified {
            println!("  {}", format!("~ {}", old_item.display_string()).yellow());
            println!("    {} {}", "-".red(), old_item.signature.red());
            println!("    {} {}", "+".green(), new_item.signature.green());
        }
        println!();
    }

    println!(
        "{}",
        format!(
            "Summary: +{} / -{} / ~{}",
            added_count, removed_count, modified_count
        )
        .bold()
    );
}

// Main diff command handler
async fn diff_docs(crate_name: &str, ver1: &str, ver2: &str) -> Result<()> {
    // Fetch both versions
    let json1 = fetch_docs_json(crate_name, ver1).await?;
    let json2 = fetch_docs_json(crate_name, ver2).await?;

    println!("Parsing API items...");

    // Extract API items from both versions
    let items1 = extract_api_items(&json1)?;
    let items2 = extract_api_items(&json2)?;

    println!("Comparing {} items...", items1.len() + items2.len());

    // Compare and categorize changes
    let (added, removed, modified) = compare_api_items(items1, items2);

    // Display results
    display_diff(crate_name, ver1, ver2, added, removed, modified);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
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

        Commands::Diff {
            crate_name,
            ver1,
            ver2,
        } => {
            diff_docs(crate_name, ver1, ver2).await?;
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
    }

    Ok(())
}
