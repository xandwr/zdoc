use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use clap::{Parser, Subcommand};
use std::path::Path;

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
        /// The crate to search within (optional)
        crate_name: Option<String>,
        /// The search term
        query: String,
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
            crate_name,
            query,
            results,
        } => {
            // logic: filter metadata.packages for crate_name if provided,
            // then search their associated rustdoc JSONs
            println!("Searching for '{}'...", query);
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
