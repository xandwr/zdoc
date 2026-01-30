# Decoded Examples from search.index/root.js

## Actual Data from Your Project

### Top-Level Structure

```json
{
  "normalizedName": {
    "I": "BQBAAAAPbAB0bAA1AEAAADJQAFYAVwBYAG0vADgAOQA6...",
    "N": "Ge",
    "E": "OjAAAAAAAAA=",
    "H": "KgliyEE8"
  },
  "crateNames": {
    "N": "a",
    "E": "OjAAAAAAAAA=",
    "H": "xV6Misrg"
  },
  "name": {
    "N": "Ge",
    "E": "OjAAAAAAAAA=",
    "H": "F3jVSDKc"
  },
  "path": {
    "N": "Bl",
    "E": "OzAAAAEAAEgAEAADAAAABgABAA4AAQATAA8AJAADACwAAQA1AAAAOAACAD4ABQBHAAAASQAGAFIAAABWAAgAYAABAGMAAABmAA4A",
    "H": "xKyY2Qtg"
  }
}
```

### Field Analysis

#### 1. normalizedName Field

**I field (compressed data):**
- Length: 4,752 characters (base64)
- Decoded: 3,564 bytes of binary data
- Contains ~117 normalized item names

**N field:** `"Ge"`
- This is the reference key
- Same as `name.N`, indicating they share a string pool

**E field:** `"OjAAAAAAAAA="`
- Decoded: `[58, 48, 0, 0, 0, 0, 0, 0]`
- As 16-bit ints: `[12346, 0, 0, 0]`
- Likely an offset or count

**H field:** `"KgliyEE8"`
- 8-character hash for integrity

#### 2. crateNames Field

**N field:** `"a"`
- Single character reference
- Different from other fields - this is the crate registry

**E field:** Same as normalizedName
- `[12346, 0, 0, 0]` suggests 12,346 items or offset

#### 3. path Field

**N field:** `"Bl"`
- Different reference from name fields
- Indicates separate string pool for paths

**E field:** `"OzAAAAEAAEgAEAADAAAABgABAA4AAQATAA8AJAADACwAAQA1AAAAOAACAD4ABQBHAAAASQAGAFIAAABWAAgAYAABAGMAAABmAA4A"`
- Decoded: 100 bytes
- Much larger than others - contains offsets for each item
- Decoded as 16-bit ints: `[0x3b, 0, 1, 0, 0, 72, 16, 3, 0, 0, 6, 1, ...]`

#### 4. desc Field

**N field:** `"Ac"`
- Another separate string pool for descriptions

**E field:** `"OzAAAAEAAGEACAAAAA0AEAACAB8ADAAtAAcANgAJAEEABQBIABEAWwAZAA=="`
- This is a RoaringBitmap!
- First byte `0x3b` indicates "bitmap with runs"
- Following bytes encode which items have empty descriptions

### Example: Individual Crate File

**File:** `target/doc/search.index/2b422b797b01.js`

**Content:**
```javascript
rn_("BQFBAAAGYwAxNwBBAEIAQwBmaVIAMUMAAD0ATQBOAE8AMUMAADwASgBLAEwAcwCEoFAAAABhIIEC8wIAAAAqADgA")
```

**Decoded (66 bytes):**
```
Hex: 05 01 41 00 00 06 63 00 31 37 00 41 00 42 00 43 00 66 69 52 00
     31 43 00 00 3d 00 4d 00 4e 00 4f 00 31 43 00 00 3c 00 4a 00 4b
     00 4c 00 73 00 84 a0 50 00 00 00 61 20 81 02 f3 02 00 00 00 2a
     00 38 00
```

**Pattern Recognition:**
- Bytes followed by 0x00 suggest null-terminated values
- Many values are in printable ASCII range (0x41='A', 0x42='B', 0x43='C')
- Structure appears to encode:
  - Version/format markers (0x05, 0x01)
  - Item counts (0x06 = 6 items?)
  - Item names or codes
  - Metadata values

### Observed Patterns

#### Pattern 1: Null-Terminated Data
Many sequences end with 0x00, suggesting null termination:
```
63 00    → 'c'
41 00    → 'A'
42 00    → 'B'
43 00    → 'C'
```

#### Pattern 2: Small Integer Sequences
Sequences like `3d 00 4d 00 4e 00 4f 00` might be:
- Indices: 61, 77, 78, 79
- Or character codes: '=', 'M', 'N', 'O'

#### Pattern 3: RoaringBitmap Headers
The desc.E field starts with `0x3b`:
```
0x3a = Standard Roaring Bitmap
0x3b = Roaring Bitmap with run-length encoding
<0x3a = Super-compressed format
>0xf0 = Special compressed format
0x00 = Empty bitmap
```

### What This Means for Searching

Based on this analysis, here's what you can extract:

1. **Item Count**: The E field first value (12,346) likely indicates total items
2. **Crate Reference**: The N field maps items to crates
3. **Path Indices**: The path.E field maps each item to its module path
4. **Empty Descriptions**: The desc.E RoaringBitmap flags items without descriptions
5. **Type Information**: Encoded in the type field (not shown in detail)

### Practical Extraction Strategy

```python
import json
import base64
import re

# Read root.js
with open('target/doc/search.index/root.js', 'r') as f:
    content = f.read()

# Extract JSON
match = re.search(r"rr_\('(.+)'\)", content, re.DOTALL)
data = json.loads(match.group(1))

# Get basic counts
def decode_e_field(e_str):
    binary = base64.b64decode(e_str)
    # First 2 bytes as little-endian uint16
    if len(binary) >= 2:
        return binary[0] | (binary[1] << 8)
    return 0

item_count = decode_e_field(data['normalizedName']['E'])
print(f"Estimated item count: {item_count}")

# Get reference keys
print(f"Name pool key: {data['normalizedName']['N']}")
print(f"Crate key: {data['crateNames']['N']}")
print(f"Path pool key: {data['path']['N']}")
print(f"Desc pool key: {data['desc']['N']}")
```

### Size Statistics

From your project:
- Root index: ~53 KB JSON (compressed)
- Individual crate indices: ~66-100 bytes each (tiny!)
- Total search.index directory: Varies by project size

The heavy compression comes from:
1. **String deduplication**: Same strings referenced by index
2. **RoaringBitmaps**: Efficient flag storage
3. **Base64 encoding**: Binary data as text
4. **Shared pools**: N field allows sharing across fields

### What You Can't Easily Extract

Without the full decoder:
- ❌ Actual item names (need I field decoder)
- ❌ Full module paths (need path.I decoder)
- ❌ Descriptions (need desc.I decoder)
- ❌ Function signatures (complex type encoding)

### What You CAN Extract

With basic decoding:
- ✅ Approximate item count
- ✅ Reference relationships (via N fields)
- ✅ Which items have no description (via RoaringBitmap)
- ✅ File structure and organization
- ✅ Crate boundaries

## Conclusion

The stringdex format is highly optimized but not designed for external parsing. For `zdoc`, the recommended approach remains:

1. Parse HTML documentation (reliable, complete)
2. OR use JavaScript runtime (fast, uses official decoder)
3. OR build from source with syn (most flexible)

Direct binary decoding is possible but requires significant reverse engineering effort and may break between rustdoc versions.
