#!/usr/bin/env python3
import json
import re
import base64

# Read the root.js file
with open('target/doc/search.index/root.js', 'r', encoding='utf-8') as f:
    content = f.read()

# Extract JSON from rr_('...')
match = re.search(r"rr_\('(.+)'\)", content, re.DOTALL)
if not match:
    print("Could not find rr_('...') pattern")
    exit(1)

json_str = match.group(1)

# Parse JSON
data = json.loads(json_str)

print("=== ROOT STRUCTURE ===\n")
for key, value in data.items():
    print(f"Key: {key}")
    if isinstance(value, dict):
        print(f"  Type: Object")
        for k, v in value.items():
            v_type = type(v).__name__
            if isinstance(v, str):
                print(f"    - {k}: {v_type} (length: {len(v)})")
            else:
                print(f"    - {k}: {v_type}")
    elif isinstance(value, list):
        print(f"  Type: List (length: {len(value)})")
    elif isinstance(value, str):
        print(f"  Type: String (length: {len(value)})")
    print()

# Analyze the compressed fields
print("\n=== ANALYZING COMPRESSED FIELDS ===\n")

for field_name in ['normalizedName', 'name', 'path', 'entry', 'desc', 'function', 'type']:
    if field_name in data:
        print(f"{field_name} structure:")
        field = data[field_name]
        if isinstance(field, dict):
            for key, value in field.items():
                if isinstance(value, str):
                    print(f"  {key}: string (length: {len(value)})")
                    if len(value) < 100:
                        print(f"      Value: {value}")
                else:
                    print(f"  {key}: {type(value).__name__}")
        print()

# Try to understand the encoding
print("\n=== DECODING ATTEMPT ===\n")

if 'normalizedName' in data and 'I' in data['normalizedName']:
    i_field = data['normalizedName']['I']
    print(f"normalizedName.I (first 200 chars):")
    print(i_field[:200])
    print()

    # Try to decode as base64
    try:
        decoded = base64.b64decode(i_field)
        print(f"Base64 decode successful! Length: {len(decoded)} bytes")
        print(f"First 100 bytes as hex:")
        print(' '.join(f'{b:02x}' for b in decoded[:100]))
        print()
        print(f"First 100 bytes as ASCII (. for non-printable):")
        print(''.join(chr(b) if 32 <= b < 127 else '.' for b in decoded[:100]))
        print()

        # Try to understand the structure
        # Look for patterns - maybe it's variable-length encoded
        print("Looking for null-terminated strings...")
        strings = []
        current = []
        for b in decoded[:500]:
            if b == 0:
                if current:
                    s = bytes(current).decode('utf-8', errors='ignore')
                    if s and len(s) > 1:
                        strings.append(s)
                    current = []
            elif 32 <= b < 127:  # printable ASCII
                current.append(b)
            else:
                if current:
                    s = bytes(current).decode('utf-8', errors='ignore')
                    if s and len(s) > 1:
                        strings.append(s)
                    current = []

        if strings:
            print(f"Found {len(strings)} potential strings:")
            for s in strings[:20]:  # First 20
                print(f"  - {s}")
    except Exception as e:
        print(f"Base64 decode failed: {e}")

# Check what N, E, H fields are
print("\n\n=== N, E, H FIELDS ===\n")
if 'normalizedName' in data:
    field = data['normalizedName']
    if 'N' in field:
        print(f"N field: {field['N']}")
    if 'E' in field:
        print(f"E field (length {len(field['E'])}): {field['E'][:100] if len(field['E']) > 100 else field['E']}")
        try:
            e_decoded = base64.b64decode(field['E'])
            print(f"  E decoded: {len(e_decoded)} bytes")
            print(f"  As integers: {list(e_decoded[:20])}")
        except:
            pass
    if 'H' in field:
        print(f"H field: {field['H']}")
        print(f"  Looks like a hash: {len(field['H'])} chars")

print("\n\n=== SAMPLE DATA FROM DIFFERENT FIELDS ===\n")

# Print samples from each compressed field to see patterns
for field_name in ['normalizedName', 'name', 'path']:
    if field_name in data:
        field = data[field_name]
        if 'I' in field and isinstance(field['I'], str):
            print(f"{field_name}.I first 300 chars:")
            print(field['I'][:300])
            print()
