# Rustdoc Search Index Analysis Summary

## What We Found

The `target/doc/search.index/root.js` file contains a highly compressed search index using a format called "stringdex" developed by the Rust project.

### File Structure

```
target/doc/search.index/
├── root.js              # Main registry with metadata
├── 2b422b797b01.js      # Individual crate indices (hash-named)
├── 08825ea748b2.js
├── 5efecfaf8a84.js
└── ...
```

### JSON Structure in root.js

```json
{
  "normalizedName": {"I": "...", "N": "Ge", "E": "...", "H": "..."},
  "crateNames": {"I": "...", "N": "a", "E": "...", "H": "..."},
  "name": {"I": "...", "N": "Ge", "E": "...", "H": "..."},
  "path": {"I": "...", "N": "Bl", "E": "...", "H": "..."},
  "entry": {"I": "...", "N": "Dm", "E": "...", "H": "..."},
  "desc": {"I": "...", "N": "Ac", "E": "...", "H": "..."},
  "function": {"I": "...", "N": "Dd", "E": "...", "H": "..."},
  "type": {"I": "...", "N": "An", "E": "...", "H": "..."},
  "alias": {"I": "...", "N": "`", "E": "...", "H": "..."},
  "generic_inverted_index": {"I": "...", "N": "b", "E": "...", "H": "..."}
}
```

Each field has:
- **I**: Base64-encoded compressed string pool data
- **N**: 1-2 character reference key
- **E**: Base64-encoded metadata (indices/offsets/RoaringBitmaps)
- **H**: 8-character integrity hash

## Challenges for Direct Decoding

1. **Proprietary Compression**: The I field uses a custom variable-length encoding scheme
2. **No Public Spec**: The exact binary format is not formally documented
3. **Minified JavaScript**: The decoder is in minified `stringdex.js`
4. **Version-Dependent**: Format may change between rustdoc versions
5. **Complex RoaringBitmaps**: The E field uses RoaringBitmap serialization

## Recommended Approaches for zdoc

### Option 1: Use Rustdoc's Search UI (Easiest)
Load the search index using rustdoc's own JavaScript:

**Pros:**
- Always compatible with current rustdoc version
- Handles all compression automatically
- No need to understand binary format

**Cons:**
- Requires JavaScript runtime (Node.js, Deno, or embedded V8)
- Dependent on rustdoc's implementation

**Implementation:**
```rust
// Use a JavaScript runtime to evaluate the search index
use deno_core::JsRuntime;

fn load_search_index(search_js_path: &Path) -> Vec<SearchItem> {
    let mut runtime = JsRuntime::new(Default::default());
    // Load stringdex.js and search.js
    // Execute root.js
    // Call JavaScript functions to get decoded data
    // Return as Rust structures
}
```

### Option 2: Parse HTML Documentation (Reliable)
Extract search data from the generated HTML:

**Pros:**
- HTML is stable and well-documented
- Can extract descriptions, signatures, etc.
- No binary format to decode

**Cons:**
- Slower than reading index directly
- More disk I/O
- May miss some metadata

**Implementation:**
```rust
use scraper::{Html, Selector};

fn extract_items_from_html(doc_dir: &Path) -> Vec<SearchItem> {
    // Walk doc directory
    // Parse HTML files
    // Extract item names, types, descriptions
    // Build search index
}
```

### Option 3: Use cargo metadata + Source Parsing (Most Robust)
Combine cargo metadata with source analysis:

**Pros:**
- Direct access to source truth
- Can get information not in docs
- No dependency on rustdoc format

**Cons:**
- More complex implementation
- Slower build time
- Need to handle proc macros

**Implementation:**
```rust
use cargo_metadata::MetadataCommand;
use syn::{parse_file, Item};

fn build_index_from_source() -> Vec<SearchItem> {
    // Use cargo metadata to find source files
    // Parse with syn crate
    // Extract public items
    // Build search index
}
```

### Option 4: Reverse Engineer Stringdex (Advanced)
Implement the stringdex decoder in Rust:

**Pros:**
- Fast, direct access to index
- No JavaScript runtime needed
- Can customize for specific needs

**Cons:**
- Complex implementation
- May break with rustdoc updates
- Significant development effort

**Implementation:**
```rust
mod stringdex {
    fn decode_roaring_bitmap(data: &[u8]) -> HashSet<usize> { /* ... */ }
    fn decode_string_pool(data: &[u8]) -> Vec<String> { /* ... */ }
    fn decode_index(root_js: &str) -> SearchIndex { /* ... */ }
}
```

## Recommendation for zdoc

**Start with Option 2 (HTML Parsing)** because:

1. ✅ Reliable and future-proof
2. ✅ HTML format is stable
3. ✅ Can extract all needed information
4. ✅ No complex binary decoding
5. ✅ Works with all rustdoc versions

**Then optimize with Option 1 (JavaScript Runtime)** if:
- Performance is critical
- You need real-time search
- You're okay with JavaScript dependency

## Practical Example for zdoc

Here's a hybrid approach combining the best of both:

```rust
use std::path::Path;
use serde_json::Value;
use regex::Regex;

// Level 1: Quick index from search.index (simplified)
pub fn quick_search(query: &str) -> Vec<String> {
    // Read root.js and individual crate files
    // Extract just the names (N fields) without full decoding
    // Do fuzzy matching on the simple data
    // Return list of matches
}

// Level 2: Detailed info from HTML
pub fn get_item_details(item_name: &str) -> ItemDetails {
    // Find the HTML file for this item
    // Parse and extract full documentation
    // Return complete item information
}

// Workflow:
// 1. User types search query
// 2. quick_search() finds matching names
// 3. Display list to user
// 4. When user selects an item, get_item_details()
```

## Fields Available in Search Index

From the analysis, here are the searchable fields:

| Field | Purpose | Useful For |
|-------|---------|------------|
| `normalizedName` | Lowercase, no underscores | Fuzzy search matching |
| `name` | Original name with case | Display to user |
| `path` | Module path | Showing item location |
| `entry` | Entry point indices | Navigation |
| `desc` | Description text | Search by description |
| `function` | Function signatures | Type-based search |
| `type` | Item type code | Filtering (fn/struct/enum) |

## Next Steps for zdoc

1. **Implement HTML parser** to extract items
2. **Build in-memory search index** with fuzzy matching
3. **Add type filters** (functions only, structs only, etc.)
4. **Cache parsed index** to speed up subsequent searches
5. **Consider JavaScript runtime** later if performance is needed

## Resources

- [Rustdoc Internals - Search](https://rustc-dev-guide.rust-lang.org/rustdoc-internals/search.html)
- [Stringdex PR #145911](https://github.com/rust-lang/rust/pull/145911)
- [Stringdex PR #147002](https://github.com/rust-lang/rust/pull/147002)
- Generated files in `target/doc/`:
  - `search.index/root.js` - Main index
  - `static.files/stringdex-*.js` - Decoder
  - `static.files/search-*.js` - Search implementation
