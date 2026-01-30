use serde_json::Value;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the root.js file
    let content = fs::read_to_string("target/doc/search.index/root.js")?;

    // Extract JSON from rr_('...')
    let start = content.find("rr_('").unwrap() + 5;
    let end = content.rfind("')").unwrap();
    let json_str = &content[start..end];

    // Parse JSON
    let data: Value = serde_json::from_str(json_str)?;

    println!("=== ROOT STRUCTURE ===\n");
    if let Value::Object(map) = &data {
        for (key, value) in map {
            println!("Key: {}", key);
            match value {
                Value::Object(obj) => {
                    println!("  Type: Object");
                    for (k, v) in obj {
                        println!("    - {}: {}", k, describe_value(v));
                    }
                }
                Value::Array(arr) => {
                    println!("  Type: Array (length: {})", arr.len());
                    if !arr.is_empty() {
                        println!("    First element: {}", describe_value(&arr[0]));
                    }
                }
                Value::String(s) => {
                    println!("  Type: String (length: {})", s.len());
                    if s.len() < 100 {
                        println!("    Value: {}", s);
                    } else {
                        println!("    Value: {}...", &s[..100]);
                    }
                }
                _ => println!("  Type: {:?}", value),
            }
            println!();
        }
    }

    // Analyze the compressed fields
    println!("\n=== ANALYZING COMPRESSED FIELDS ===\n");

    if let Some(obj) = data.as_object() {
        // Look at normalizedName structure
        if let Some(normalized_name) = obj.get("normalizedName") {
            println!("normalizedName structure:");
            analyze_compressed_field(normalized_name);
        }

        if let Some(name) = obj.get("name") {
            println!("\nname structure:");
            analyze_compressed_field(name);
        }

        if let Some(path) = obj.get("path") {
            println!("\npath structure:");
            analyze_compressed_field(path);
        }

        if let Some(entry) = obj.get("entry") {
            println!("\nentry structure:");
            analyze_compressed_field(entry);
        }

        if let Some(desc) = obj.get("desc") {
            println!("\ndesc structure:");
            analyze_compressed_field(desc);
        }
    }

    // Try to decode the base64-like strings
    println!("\n=== ATTEMPTING TO DECODE ===\n");

    if let Some(obj) = data.as_object() {
        if let Some(normalized_name) = obj.get("normalizedName") {
            if let Some(i_field) = normalized_name.get("I") {
                if let Some(i_str) = i_field.as_str() {
                    println!("normalizedName.I (first 200 chars):");
                    println!("{}", &i_str[..i_str.len().min(200)]);
                    println!("\nAttempting to decode as base64...");

                    // Try decoding
                    if let Ok(decoded) = base64_decode_custom(i_str) {
                        println!("Decoded length: {} bytes", decoded.len());
                        println!("First 50 bytes as hex: {}",
                            decoded.iter()
                                .take(50)
                                .map(|b| format!("{:02x}", b))
                                .collect::<Vec<_>>()
                                .join(" "));
                        println!("First 50 bytes as ASCII (. for non-printable): {}",
                            decoded.iter()
                                .take(50)
                                .map(|&b| if b >= 32 && b < 127 { b as char } else { '.' })
                                .collect::<String>());
                    }
                }
            }
        }
    }

    Ok(())
}

fn describe_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("bool: {}", b),
        Value::Number(n) => format!("number: {}", n),
        Value::String(s) => {
            if s.len() < 50 {
                format!("string: \"{}\"", s)
            } else {
                format!("string (len {}): \"{}...\"", s.len(), &s[..47])
            }
        }
        Value::Array(a) => format!("array (len {})", a.len()),
        Value::Object(o) => format!("object (keys: {})", o.keys().map(|k| k.as_str()).collect::<Vec<_>>().join(", ")),
    }
}

fn analyze_compressed_field(field: &Value) {
    if let Value::Object(obj) = field {
        for (key, value) in obj {
            println!("  {}: {}", key, describe_value(value));
        }
    }
}

fn base64_decode_custom(s: &str) -> Result<Vec<u8>, String> {
    // This appears to be a custom base64-like encoding
    // Standard base64 alphabet: ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/

    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.decode(s).map_err(|e| e.to_string())
}
