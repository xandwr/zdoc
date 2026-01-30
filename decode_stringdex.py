#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Decode the rustdoc search index format (stringdex compression)

The format appears to use:
- I: The main compressed data (base64 encoded binary data)
- N: A short string (maybe a reference or key)
- E: Encoding metadata (base64, appears to contain offsets/lengths)
- H: Hash for integrity

The I field contains variable-length encoded data with:
- Null bytes (0x00) as delimiters
- UTF-16-like encoding (two bytes per character) for some strings
- Raw ASCII strings
"""

import json
import re
import base64
import struct
import sys

# Force UTF-8 output
if sys.platform == 'win32':
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')

def read_varint(data, offset):
    """Read a variable-length integer (LEB128-like encoding)"""
    result = 0
    shift = 0
    while offset < len(data):
        byte = data[offset]
        offset += 1
        result |= (byte & 0x7F) << shift
        if (byte & 0x80) == 0:
            break
        shift += 7
    return result, offset

def decode_utf16_null_terminated(data, offset):
    """Read a null-terminated UTF-16LE string"""
    chars = []
    while offset + 1 < len(data):
        low = data[offset]
        high = data[offset + 1]
        offset += 2
        if low == 0 and high == 0:
            break
        chars.append(chr(low | (high << 8)))
    return ''.join(chars), offset

def extract_strings_from_i_field(i_data):
    """Extract strings from the I field binary data"""
    strings = []
    i = 0

    while i < len(i_data):
        # Try to read as UTF-16LE
        if i + 1 < len(i_data):
            # Check if this looks like UTF-16LE (low byte has value, high byte is 0)
            low = i_data[i]
            high = i_data[i + 1] if i + 1 < len(i_data) else 0

            # If we see printable ASCII followed by null byte, might be UTF-16
            if 32 <= low < 127 and high == 0:
                s, new_i = decode_utf16_null_terminated(i_data, i)
                if s and len(s) > 1:
                    strings.append(s)
                    i = new_i
                    continue

        i += 1

    return strings

def analyze_e_field(e_data):
    """Analyze the E field which seems to contain offsets or metadata"""
    decoded = base64.b64decode(e_data)

    # Try to interpret as array of shorts (16-bit integers)
    if len(decoded) % 2 == 0:
        shorts = []
        for i in range(0, len(decoded), 2):
            val = struct.unpack('<H', decoded[i:i+2])[0]
            shorts.append(val)
        return shorts
    return list(decoded)

# Read the root.js file
with open('target/doc/search.index/root.js', 'r', encoding='utf-8') as f:
    content = f.read()

# Extract JSON from rr_('...')
match = re.search(r"rr_\('(.+)'\)", content, re.DOTALL)
json_str = match.group(1)
data = json.loads(json_str)

print("=== RUSTDOC SEARCH INDEX ANALYSIS ===\n")

# Decode the normalizedName field
print("1. NORMALIZED NAMES:")
print("-" * 60)
if 'normalizedName' in data:
    i_field = data['normalizedName']['I']
    i_decoded = base64.b64decode(i_field)

    print(f"Compressed size: {len(i_field)} chars")
    print(f"Decoded size: {len(i_decoded)} bytes")
    print(f"N field: {data['normalizedName']['N']}")
    print(f"H field (hash): {data['normalizedName']['H']}")
    print()

    strings = extract_strings_from_i_field(i_decoded)
    print(f"Extracted {len(strings)} normalized names:")
    for i, s in enumerate(strings[:30]):  # First 30
        try:
            print(f"  {i:3d}. {s}")
        except UnicodeEncodeError:
            print(f"  {i:3d}. {repr(s)}")
    if len(strings) > 30:
        print(f"  ... and {len(strings) - 30} more")

print("\n2. REGULAR NAMES:")
print("-" * 60)
if 'name' in data:
    # name.N appears to be the same as normalizedName.N, suggesting they share data
    print(f"N field: {data['name']['N']}")
    print(f"H field (hash): {data['name']['H']}")
    print("(Note: N field matches normalizedName, likely references same string pool)")

print("\n3. PATHS:")
print("-" * 60)
if 'path' in data:
    i_field = data['path']['I']
    i_decoded = base64.b64decode(i_field)

    print(f"N field: {data['path']['N']}")
    print(f"Compressed size: {len(i_field)} chars")
    print(f"Decoded size: {len(i_decoded)} bytes")
    print()

    strings = extract_strings_from_i_field(i_decoded)
    print(f"Extracted {len(strings)} paths:")
    for i, s in enumerate(strings[:20]):
        try:
            print(f"  {i:3d}. {s}")
        except UnicodeEncodeError:
            print(f"  {i:3d}. {repr(s)}")

print("\n4. ENTRY POINTS:")
print("-" * 60)
if 'entry' in data:
    print(f"N field: {data['entry']['N']}")
    e_field_data = analyze_e_field(data['entry']['E'])
    print(f"E field decoded (first 20 values): {e_field_data[:20]}")
    print("(These are likely indices or offsets)")

print("\n5. DESCRIPTIONS:")
print("-" * 60)
if 'desc' in data:
    print(f"N field: {data['desc']['N']}")
    e_field_data = analyze_e_field(data['desc']['E'])
    print(f"E field decoded (first 20 values): {e_field_data[:20]}")

print("\n6. TYPES:")
print("-" * 60)
if 'type' in data:
    print(f"N field: {data['type']['N']}")
    print("Type field contains type signatures for functions")

print("\n\n=== UNDERSTANDING THE FORMAT ===\n")
print("""
This is rustdoc's new compressed search index format called "stringdex".

Structure:
- Each field (normalizedName, name, path, etc.) has:
  * I: Base64-encoded binary data containing the actual strings/data
       Strings are stored as UTF-16LE, null-terminated
  * N: A short reference key (seems to reference a crate or section)
  * E: Encoded metadata (indices, offsets, or lengths)
  * H: Hash for integrity checking

The strings are compressed in the I field using UTF-16LE encoding with
null terminators. The E field appears to contain indices that map items
to their positions in the string pool.

To build a search index:
1. Decode the I field from base64
2. Extract null-terminated UTF-16LE strings
3. Use the E field to map items to their string indices
4. The N field likely identifies which crate/section
5. Cross-reference between name, path, entry, desc fields using indices
""")

print("\n=== EXTRACTING SEARCHABLE ITEMS ===\n")
print("Looking for crate-specific data...\n")

# Try to find crate names
if 'crateNames' in data:
    print("Crate names:")
    print(f"  N: {data['crateNames']['N']}")
    e_data = analyze_e_field(data['crateNames']['E'])
    print(f"  E field: {e_data}")
