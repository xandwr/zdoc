# Rustdoc Search Index Format (Stringdex)

## Overview

Rustdoc uses a compressed search index format called "stringdex" to minimize the size of documentation search indices. The format stores data in column-major form with parallel arrays, keeping data compressed even in memory at runtime.

## File Structure

### Root Index (`target/doc/search.index/root.js`)

The root index file contains a JavaScript function call that initializes the search registry:

```javascript
rr_('{"normalizedName":{...},"crateNames":{...},"name":{...},...}')
```

The JSON object contains these top-level fields:
- `normalizedName` - Normalized item names (lowercase, no underscores)
- `crateNames` - Crate name mappings
- `name` - Original item names (with case and underscores)
- `path` - Module paths for items
- `entry` - Entry point indices
- `desc` - Item descriptions
- `function` - Function signatures
- `type` - Type information
- `alias` - Type aliases
- `generic_inverted_index` - Generic parameter search index

### Individual Crate Files

Each crate has its own index file named by hash (e.g., `2b422b797b01.js`):

```javascript
rn_("BQFBAAAGYwAxNwBBAEIAQwBmaVIAMUMAAD0ATQBOAE8AMUMAADwASgBLAEwAcwCEoFAAAABhIIEC8wIAAAAqADgA")
```

The base64 string contains compressed crate-specific data.

## Field Structure

Each field (normalizedName, name, path, etc.) is an object with four properties:

### I - Index Data (String Pool)
- Base64-encoded binary data containing the actual strings/values
- Uses variable-length encoding and compression
- Strings remain compressed in memory until accessed

### N - Name/Reference Key
- Short string (1-2 characters) that acts as a reference key
- Used to identify which crate or section this data belongs to
- Example: `"Ge"`, `"a"`, `"Bl"`

### E - Encoding Metadata
- Base64-encoded binary data containing indices, offsets, or lengths
- Used to map items to their positions in the string pool
- For the "desc" field, this is a RoaringBitmap indicating which items have empty descriptions

### H - Hash
- 8-character hash for integrity checking
- Example: `"KgliyEE8"`

## Data Compression Techniques

### 1. RoaringBitmaps
Used for flag-style data like deprecation status and empty descriptions:
- Serialized in Roaring Bitmap format with run-length encoding
- Base64 encoded for transport
- Stays compressed in memory

Format bytes:
- `0x3a` or `0x3b` - Standard Roaring Bitmap with optional runs
- `0x00` - Empty bitmap
- `>0xf0` - Special compressed format
- `<0x3a` - Super-compressed format for small sets

### 2. VLQ Hex Encoding
Variable-Length Quantity encoding used for function signatures and sparse data:
- Allows efficient storage of variable-sized integers
- Keeps data compressed in memory during runtime

### 3. Column-Major Storage
Data is organized in parallel arrays (columns) where:
- Same index across different arrays refers to the same item
- More cache-efficient than row-major for search operations
- Enables compression of similar data together

## Item Type Codes

Items are identified by single-character or numeric type codes:

| Code | Type          | Code | Type          |
|------|---------------|------|---------------|
| 0    | mod           | 12   | macro         |
| 1    | struct        | 13   | primitive     |
| 2    | enum          | 14   | assoc-type    |
| 3    | fn            | 15   | constant      |
| 4    | type          | 16   | assoc-const   |
| 5    | static        | 17   | union         |
| 6    | trait         | 18   | foreign-type  |
| 7    | impl          | 19   | keyword       |
| 8    | tymethod      | 20   | existential   |
| 9    | method        | 21   | attr          |
| 10   | structfield   | 22   | derive        |
| 11   | variant       | 23   | trait-alias   |

## Decoding Strategy

To build a search index from this format:

### 1. Parse the JSON Structure
```python
import json
import re
import base64

with open('target/doc/search.index/root.js', 'r') as f:
    content = f.read()

# Extract JSON from rr_('...')
match = re.search(r"rr_\('(.+)'\)", content, re.DOTALL)
data = json.loads(match.group(1))
```

### 2. Decode RoaringBitmaps
The E field for desc/flags contains RoaringBitmaps:
```python
def decode_roaring_bitmap(base64_str):
    binary = base64.b64decode(base64_str)
    # Parse RoaringBitmap format (see implementation in stringdex.js)
    # Returns set of indices that have the flag set
```

### 3. Decode String Pools
The I field contains compressed string data. The exact decompression algorithm is implemented in rustdoc's JavaScript but involves:
- Base64 decoding the I field
- Parsing variable-length encoded data structures
- Using the E field to map indices to strings

### 4. Build Search Index
```rust
// Pseudo-code for building a searchable index
struct SearchIndex {
    items: Vec<SearchItem>,
}

struct SearchItem {
    name: String,              // from 'name' field
    normalized_name: String,   // from 'normalizedName' field
    path: String,              // from 'path' field
    item_type: u8,             // from 'type' field
    description: Option<String>, // from 'desc' field (if not in empty bitmap)
    crate_name: String,        // from 'crateNames' field
}
```

## Size Optimizations

Recent improvements (2026):
- Standard library docs: 18M → 16M (11% reduction)
- Compiler docs: 57M → 49M (14% reduction)
- Achieved through better string packing and compression

## Implementation References

For the canonical implementation, see:
- `rustc-dev-guide`: Rustdoc search internals documentation
- `src/librustdoc/html/static/js/search.js`: Search implementation
- `src/librustdoc/html/static/js/stringdex.js`: Decoding algorithms

## Building a Fuzzy Search Tool

For your `zdoc` tool to enable fuzzy searching:

1. **Parse the root.js** to get the registry of all items
2. **Extract normalized names** for matching (lowercase, no underscores)
3. **Use fuzzy matching** (like SkimMatcherV2) on normalized names
4. **Map back to original names** for display
5. **Include type filters** to search only functions, structs, etc.
6. **Load individual crate files** on-demand for detailed information

The N field acts as a cross-reference key - when you see the same N value across different fields, they refer to the same string pool, allowing you to correlate data without duplicating strings.

## Caveats

- The exact binary format of the I field is not formally documented
- The decoding logic is implemented in minified JavaScript
- rustdoc may change the format between versions
- Always test against the rustdoc version you're targeting

For production use, consider:
- Extracting strings using rustdoc's own search.js
- Running cargo doc and parsing the resulting HTML
- Using cargo metadata for structural information
- Falling back to parsing Rust source if index is unavailable
