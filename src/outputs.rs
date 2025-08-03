use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;

/// Represents an output reference like ${resourceName.outputProperty}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutputReference {
    pub resource_name: String,
    pub property_path: String,
}

impl OutputReference {
    /// Parse a string like "resourceName.property.nested" into OutputReference
    pub fn parse(reference: &str) -> Option<Self> {
        let parts: Vec<&str> = reference.split('.').collect();
        if parts.len() >= 2 {
            Some(OutputReference {
                resource_name: parts[0].to_string(),
                property_path: parts[1..].join("."),
            })
        } else {
            None
        }
    }
}

/// Finds all output references in a JSON value
pub fn find_output_references(value: &serde_json::Value) -> Vec<OutputReference> {
    let mut references = Vec::new();
    find_references_recursive(value, &mut references);
    references
}

fn find_references_recursive(value: &serde_json::Value, references: &mut Vec<OutputReference>) {
    match value {
        serde_json::Value::String(s) => {
            // Look for ${...} patterns
            let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
            for cap in re.captures_iter(s) {
                if let Some(reference) = OutputReference::parse(&cap[1]) {
                    references.push(reference);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                find_references_recursive(item, references);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, value) in map {
                find_references_recursive(value, references);
            }
        }
        _ => {}
    }
}

/// Resolves output references in a JSON value using provided outputs
pub fn resolve_output_references(
    value: &serde_json::Value,
    outputs: &HashMap<String, serde_json::Value>,
) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
            let mut result = s.clone();
            
            for cap in re.captures_iter(s) {
                let full_match = &cap[0];
                let reference_str = &cap[1];
                
                if let Some(reference) = OutputReference::parse(reference_str) {
                    if let Some(resource_outputs) = outputs.get(&reference.resource_name) {
                        if let Some(value) = get_nested_value(resource_outputs, &reference.property_path) {
                            // Convert the value to string for replacement
                            let replacement = match value {
                                serde_json::Value::String(s) => s.clone(),
                                _ => value.to_string(),
                            };
                            result = result.replace(full_match, &replacement);
                        }
                    }
                }
            }
            
            serde_json::Value::String(result)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(
                arr.iter()
                    .map(|v| resolve_output_references(v, outputs))
                    .collect(),
            )
        }
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(k.clone(), resolve_output_references(v, outputs));
            }
            serde_json::Value::Object(new_map)
        }
        _ => value.clone(),
    }
}

/// Gets a nested value from a JSON object using dot notation
fn get_nested_value<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;
    
    for part in parts {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(part)?;
            }
            _ => return None,
        }
    }
    
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_output_reference() {
        let ref1 = OutputReference::parse("myResource.id").unwrap();
        assert_eq!(ref1.resource_name, "myResource");
        assert_eq!(ref1.property_path, "id");
        
        let ref2 = OutputReference::parse("myResource.properties.name").unwrap();
        assert_eq!(ref2.resource_name, "myResource");
        assert_eq!(ref2.property_path, "properties.name");
    }

    #[test]
    fn test_find_output_references() {
        let value = json!({
            "name": "${rg.name}",
            "location": "${rg.location}",
            "nested": {
                "id": "${storage.id}"
            },
            "array": ["${vm.ip}", "static-value"]
        });
        
        let refs = find_output_references(&value);
        assert_eq!(refs.len(), 4);
    }

    #[test]
    fn test_resolve_output_references() {
        let value = json!({
            "resourceGroupName": "${rg.name}",
            "location": "${rg.location}"
        });
        
        let mut outputs = HashMap::new();
        outputs.insert("rg".to_string(), json!({
            "name": "my-resource-group",
            "location": "eastus"
        }));
        
        let resolved = resolve_output_references(&value, &outputs);
        assert_eq!(resolved["resourceGroupName"], "my-resource-group");
        assert_eq!(resolved["location"], "eastus");
    }
}