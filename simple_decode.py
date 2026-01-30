#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import json
import re
import base64
import sys
import io

# Force UTF-8 output
if sys.platform == 'win32':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')

# Read the root.js file
with open('target/doc/search.index/root.js', 'r', encoding='utf-8') as f:
    content = f.read()

# Extract JSON
match = re.search(r"rr_\('(.+)'\)", content, re.DOTALL)
json_str = match.group(1)
data = json.loads(json_str)

# Write full structure to a file for inspection
with open('search_index_structure.json', 'w', encoding='utf-8') as f:
    json.dump(data, f, indent=2, ensure_ascii=False)

print("=== SEARCH INDEX STRUCTURE ===")
print(f"Saved full structure to: search_index_structure.json")
print()
print("Top-level keys:")
for key in data.keys():
    print(f"  - {key}")

print("\n=== KEY OBSERVATIONS ===\n")

# Let's check the simple crate index first
with open('target/doc/search.index/2b422b797b01.js', 'r', encoding='utf-8') as f:
    simple = f.read()

# This simpler file might give us clues
print("Sample crate index file (2b422b797b01.js):")
print(simple[:500])
print()

# Extract from it
if "rn_(" in simple:
    match2 = re.search(r'rn_\("(.+)"\)', simple, re.DOTALL)
    if match2:
        simple_data = match2.group(1)
        print("Simple file contains base64 data:")
        print(simple_data)
        print()

        decoded = base64.b64decode(simple_data)
        print(f"Decoded length: {len(decoded)} bytes")
        print("Hex dump (first 100 bytes):")
        print(' '.join(f'{b:02x}' for b in decoded[:100]))
        print()

print("\n=== LOOKING FOR STRINGDEX DECODER ===")
print("The format appears to be called 'stringdex' - let's check rustdoc source")
print()

# Analyze the E field more carefully
if 'normalizedName' in data:
    e_field = data['normalizedName']['E']
    e_decoded = base64.b64decode(e_field)
    print("normalizedName.E field:")
    print(f"  Length: {len(e_decoded)} bytes")
    print(f"  Hex: {' '.join(f'{b:02x}' for b in e_decoded)}")
    print(f"  As 16-bit ints: {[int.from_bytes(e_decoded[i:i+2], 'little') for i in range(0, len(e_decoded), 2)]}")

    n_field = data['normalizedName']['N']
    print(f"\nnormalizedName.N field: {n_field}")

# Check if there's actual crate data stored differently
print("\n\n=== STRATEGY ===")
print("""
The rustdoc format appears to use a compressed string pool format called "stringdex".

Based on the structure:
1. Fields like 'normalizedName', 'name', 'path' etc contain compressed data
2. Each has I (data), N (key), E (metadata), H (hash) subfields
3. The N field appears to be a short key that references a string pool
4. The I field contains the compressed string data
5. The E field contains indices/offsets

To properly decode this, we need to:
1. Understand the stringdex compression algorithm
2. Look at how rustdoc's search.js actually decodes this
3. Or find the rustdoc source code that generates this format

The root.js file acts as a registry that points to individual crate files like
2b422b797b01.js. Each crate file contains its own compressed data.
""")
