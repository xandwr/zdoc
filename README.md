# zdoc
A lean, terminal-first Rust documentation parser.

## What this is
I always found `cargo doc` frustrating. I mean, the command is RIGHT THERE.
What if I want to fuzzy find a search query inline instead of open a web browser?

Just rolling with the vision and seeing what emerges.

## How it works
`zdoc` only runs inside a valid Rust project. It doesn't exist in a global context.
Why would you be searching versioned Rust crate docs from your CLI outside of a Rust project? Lol.

Makes the constraint surface for the search functionality dead simple.
Just looking at what crates are actually present in the local `Cargo.toml` file.

> "Error: No `Cargo.toml` found. `zdoc` is designed to run within a Rust project to provide version-accurate documentation."
User will know what this means since... they've likely installed it via Cargo itself.

## Commands

### `search <query> [crate] {--results N}`
**Status: ✅ Implemented**

Fuzzy searches through your project's documentation.

```bash
zdoc search Command           # Search all workspace crates
zdoc search Result -r 10      # Show top 10 results
zdoc search sear              # Fuzzy matching works!
```

Returns the top N scored fuzzy results for the given query. Shows:
- Item name and type (function, struct, enum, etc.)
- Crate name
- Documentation preview (first 100 chars)

**Implementation Notes:**
- Uses `cargo doc` with JSON output format (`-Z unstable-options --output-format json`)
- Works on stable Rust via `RUSTC_BOOTSTRAP=1` (enables unstable rustdoc features)
- Parses generated `target/doc/{crate}.json` files directly
- Uses `fuzzy-matcher` crate (SkimMatcherV2) for fast local fuzzy matching
- No nightly Rust required!

**Why JSON format?**
The HTML-based `search-index.js` format changed to a compressed "stringdex" format in recent rustdoc versions (2026+), using RoaringBitmaps and custom encoding. The JSON format is:
- Stable and well-documented (RFC 2963)
- Easy to parse with `serde_json`
- Provides structured data (item types, docs, paths)
- Future-proof (official unstable API)

**Critical:** The JSON format requires `RUSTDOCFLAGS="-Z unstable-options --output-format json"`. This is an unstable rustdoc feature but works reliably on stable Rust with `RUSTC_BOOTSTRAP=1`.

### `diff <crate> <ver1> <ver2>`
**Status: 🚧 Planned**

Pulls docs for both versions and shows a terminal diff of the public API.
Output is similar to `git diff`, additions are green and removals are red.

### `features <crate>`
**Status: ✅ Implemented**

A quick way to list the available features for the provided crate.

## Technical Details

### Search Index Format (as of Rust 1.93.0+)
Modern rustdoc generates two formats:
1. **HTML format** (`search.index/` directory): Compressed stringdex format for web browser
   - Uses RoaringBitmaps for compression
   - Custom base64 encoding with variable-length integers
   - Not documented or stable for external parsing
   - Size-optimized (18MB → 16MB for stdlib)

2. **JSON format** (`{crate}.json` files): Structured documentation data
   - Enabled via `-Z unstable-options --output-format json`
   - Defined in RFC 2963
   - Contains full item index with names, types, docs, paths
   - Designed for programmatic access

### Dependencies
- `cargo_metadata` - Parse Cargo.toml and project metadata
- `clap` - CLI argument parsing
- `fuzzy-matcher` - Fast fuzzy string matching (SkimMatcherV2 algorithm)
- `serde_json` - JSON parsing for rustdoc output
- `rustdoc-types` - Type definitions for rustdoc JSON (currently unused but available)
- `anyhow` - Error handling

### Future Considerations
- Could add caching of parsed JSON to speed up repeated searches
- Module path reconstruction is currently skipped for simplicity
- May want to add filtering by item type (functions only, structs only, etc.)
- JSON format may stabilize in future Rust versions, removing need for `RUSTC_BOOTSTRAP`