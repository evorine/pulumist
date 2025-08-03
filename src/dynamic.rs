use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::os::raw::c_char;
use prost::Message;
use crate::{proto, FreeAllocation, PulumiDynamicDeploy, PulumiDynamicDestroy, PulumiDynamicGetOutputs, PulumiDynamicPreview, PulumiDynamicRefresh};

// Dynamic resource representation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DynamicResource {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub name: String,
    pub properties: Value, // Using serde_json::Value for dynamic properties
    pub options: Option<ResourceOptions>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ResourceOptions {
    pub parent: Option<String>,
    #[serde(rename = "dependsOn")]
    pub depends_on: Option<Vec<String>>,
    pub provider: Option<String>,
    #[serde(rename = "deleteBeforeReplace")]
    pub delete_before_replace: Option<bool>,
}

// Stack request for operations
#[derive(Debug, Serialize)]
pub struct StackRequest {
    pub project: String,
    pub stack: String,
    pub backend: Option<String>,
    pub config: serde_json::Map<String, Value>,
    pub resources: Vec<DynamicResource>,
}

// Import request for importing existing resources
#[derive(Debug, Serialize)]
pub struct ImportRequest {
    pub project: String,
    pub stack: String,
    pub backend: Option<String>,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    #[serde(rename = "resourceName")]
    pub resource_name: String,
    #[serde(rename = "resourceId")]
    pub resource_id: String,
    pub resources: Vec<DynamicResource>,
    pub config: serde_json::Map<String, Value>,
    pub outputs: serde_json::Map<String, Value>,
}


// Safe wrapper around FFI calls
#[derive(Clone)]
pub struct PulumiDynamic;

impl PulumiDynamic {
    pub fn new() -> Self {
        PulumiDynamic
    }

    // Call Go function with protobuf and handle response
    fn call_go_function_pb(
        func: unsafe extern "C" fn(*const c_char, i32) -> *mut c_char,
        request: &proto::pulumist::PulumiRequest,
    ) -> Result<proto::pulumist::PulumiResponse, String> {
        let request_bytes = request.encode_to_vec();
        let request_len = request_bytes.len() as i32;

        let response_ptr = unsafe {
            func(request_bytes.as_ptr() as *const c_char, request_len)
        };

        if response_ptr.is_null() {
            return Err("Received null response from Go".to_string());
        }

        // Read the length prefix (4 bytes little-endian)
        let length_bytes = unsafe {
            std::slice::from_raw_parts(response_ptr as *const u8, 4)
        };
        let response_len = u32::from_le_bytes([
            length_bytes[0], length_bytes[1], length_bytes[2], length_bytes[3]
        ]) as usize;

        // Read the protobuf data
        let response_bytes = unsafe {
            std::slice::from_raw_parts((response_ptr as *const u8).offset(4), response_len)
        };

        let response = proto::pulumist::PulumiResponse::decode(response_bytes)
            .map_err(|e| format!("Failed to decode protobuf response: {}", e))?;

        unsafe { FreeAllocation(response_ptr); }

        Ok(response)
    }

    /// Performs a preview (dry-run) of infrastructure changes.
    ///
    /// Shows what resources would be created, updated, or deleted
    /// without actually making changes.
    ///
    /// # Arguments
    /// * `request` - Stack configuration including resources to preview
    ///
    /// # Returns
    /// * `Ok(Value)` - JSON value with preview results
    /// * `Err(String)` - Error message if preview fails
    ///
    /// # Production Improvements
    /// - Add timeout support
    /// - Return typed PreviewResponse instead of Value
    /// - Add progress callback for long operations
    pub fn preview(&self, request: StackRequest) -> Result<Value, String> {
        // Convert StackRequest to protobuf
        let pb_request = proto::pulumist::PulumiRequest {
            working_dir: request.project.clone(),
            stack_name: request.stack.clone(),
            project_name: request.project.clone(),
            resources: request.resources.into_iter().map(|r| {
                proto::pulumist::Resource {
                    r#type: r.resource_type,
                    name: r.name,
                    properties: self.json_to_pb_map(&r.properties),
                    depends_on: r.options.as_ref()
                        .and_then(|o| o.depends_on.clone())
                        .unwrap_or_default(),
                    provider: r.options.as_ref()
                        .and_then(|o| o.provider.clone())
                        .unwrap_or_default(),
                }
            }).collect(),
            config: request.config.into_iter()
                .map(|(k, v)| (k, v.as_str().unwrap_or("").to_string()))
                .collect(),
            pulumi_config: None,
        };

        let response = Self::call_go_function_pb(PulumiDynamicPreview, &pb_request)?;

        if response.success {
            // Convert outputs to JSON value
            let mut result = serde_json::Map::new();
            for output in response.outputs {
                if let Some(value) = output.value {
                    result.insert(
                        format!("{}.{}", output.resource_name, output.output_name),
                        self.pb_value_to_json(&value),
                    );
                }
            }
            Ok(Value::Object(result))
        } else {
            Err(response.error)
        }
    }

    pub fn deploy(&self, request: StackRequest) -> Result<Value, String> {
        // Convert StackRequest to protobuf
        let pb_request = proto::pulumist::PulumiRequest {
            working_dir: request.project.clone(),
            stack_name: request.stack.clone(),
            project_name: request.project.clone(),
            resources: request.resources.into_iter().map(|r| {
                proto::pulumist::Resource {
                    r#type: r.resource_type,
                    name: r.name,
                    properties: self.json_to_pb_map(&r.properties),
                    depends_on: r.options.as_ref()
                        .and_then(|o| o.depends_on.clone())
                        .unwrap_or_default(),
                    provider: r.options.as_ref()
                        .and_then(|o| o.provider.clone())
                        .unwrap_or_default(),
                }
            }).collect(),
            config: request.config.into_iter()
                .map(|(k, v)| (k, v.as_str().unwrap_or("").to_string()))
                .collect(),
            pulumi_config: None,
        };

        let response = Self::call_go_function_pb(PulumiDynamicDeploy, &pb_request)?;

        if response.success {
            // Convert outputs to JSON value
            let mut result = serde_json::Map::new();
            for output in response.outputs {
                if let Some(value) = output.value {
                    result.insert(
                        format!("{}.{}", output.resource_name, output.output_name),
                        self.pb_value_to_json(&value),
                    );
                }
            }
            Ok(Value::Object(result))
        } else {
            Err(response.error)
        }
    }

    // Helper to convert JSON to protobuf map
    fn json_to_pb_map(&self, value: &Value) -> std::collections::HashMap<String, proto::pulumist::Value> {
        let mut map = std::collections::HashMap::new();
        if let Value::Object(obj) = value {
            for (k, v) in obj {
                map.insert(k.clone(), self.json_to_pb_value(v));
            }
        }
        map
    }

    // Helper to convert JSON value to protobuf value
    fn json_to_pb_value(&self, value: &Value) -> proto::pulumist::Value {
        use proto::pulumist::value::Value as PbValue;

        match value {
            Value::String(s) => proto::pulumist::Value {
                value: Some(PbValue::StringValue(s.clone())),
            },
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    proto::pulumist::Value {
                        value: Some(PbValue::IntValue(i)),
                    }
                } else if let Some(f) = n.as_f64() {
                    proto::pulumist::Value {
                        value: Some(PbValue::DoubleValue(f)),
                    }
                } else {
                    proto::pulumist::Value {
                        value: Some(PbValue::StringValue(n.to_string())),
                    }
                }
            },
            Value::Bool(b) => proto::pulumist::Value {
                value: Some(PbValue::BoolValue(*b)),
            },
            Value::Array(arr) => proto::pulumist::Value {
                value: Some(PbValue::ListValue(proto::pulumist::ValueList {
                    values: arr.iter().map(|v| self.json_to_pb_value(v)).collect(),
                })),
            },
            Value::Object(obj) => proto::pulumist::Value {
                value: Some(PbValue::MapValue(proto::pulumist::ValueMap {
                    fields: obj.iter()
                        .map(|(k, v)| (k.clone(), self.json_to_pb_value(v)))
                        .collect(),
                })),
            },
            Value::Null => proto::pulumist::Value { value: None },
        }
    }

    // Helper to convert protobuf value to JSON
    fn pb_value_to_json(&self, value: &proto::pulumist::Value) -> Value {
        use proto::pulumist::value::Value as PbValue;

        match &value.value {
            Some(PbValue::StringValue(s)) => Value::String(s.clone()),
            Some(PbValue::IntValue(i)) => Value::Number(serde_json::Number::from(*i)),
            Some(PbValue::DoubleValue(f)) => {
                serde_json::Number::from_f64(*f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            },
            Some(PbValue::BoolValue(b)) => Value::Bool(*b),
            Some(PbValue::ListValue(list)) => Value::Array(
                list.values.iter().map(|v| self.pb_value_to_json(v)).collect()
            ),
            Some(PbValue::MapValue(map)) => Value::Object(
                map.fields.iter()
                    .map(|(k, v)| (k.clone(), self.pb_value_to_json(v)))
                    .collect()
            ),
            Some(PbValue::BytesValue(bytes)) => {
                // Convert bytes to base64 string for JSON
                use base64::{Engine as _, engine::general_purpose};
                Value::String(general_purpose::STANDARD.encode(bytes))
            },
            None => Value::Null,
        }
    }

    /// Destroys all resources in the specified stack.
    ///
    /// This operation:
    /// 1. Deletes all resources in dependency order
    /// 2. Removes stack state
    /// 3. Is NOT reversible - use with caution
    ///
    /// # Arguments
    /// * `request` - Stack configuration to destroy
    ///
    /// # Returns
    /// * `Ok(Value)` - JSON value with destruction results
    /// * `Err(String)` - Error message if destruction fails
    ///
    /// # Safety
    /// This permanently deletes infrastructure. Always preview first
    /// and ensure you have backups if needed.
    pub fn destroy(&self, request: StackRequest) -> Result<Value, String> {
        // Convert StackRequest to protobuf
        let pb_request = proto::pulumist::PulumiRequest {
            working_dir: request.project.clone(),
            stack_name: request.stack.clone(),
            project_name: request.project.clone(),
            resources: request.resources.into_iter().map(|r| {
                proto::pulumist::Resource {
                    r#type: r.resource_type,
                    name: r.name,
                    properties: self.json_to_pb_map(&r.properties),
                    depends_on: r.options.as_ref()
                        .and_then(|o| o.depends_on.clone())
                        .unwrap_or_default(),
                    provider: r.options.as_ref()
                        .and_then(|o| o.provider.clone())
                        .unwrap_or_default(),
                }
            }).collect(),
            config: request.config.into_iter()
                .map(|(k, v)| (k, v.as_str().unwrap_or("").to_string()))
                .collect(),
            pulumi_config: None,
        };

        let response = Self::call_go_function_pb(PulumiDynamicDestroy, &pb_request)?;

        if response.success {
            // Convert outputs to JSON value
            let mut result = serde_json::Map::new();
            for output in response.outputs {
                if let Some(value) = output.value {
                    result.insert(
                        format!("{}.{}", output.resource_name, output.output_name),
                        self.pb_value_to_json(&value),
                    );
                }
            }
            Ok(Value::Object(result))
        } else {
            Err(response.error)
        }
    }

    pub fn get_outputs(&self, request: StackRequest) -> Result<Value, String> {
        // Convert StackRequest to protobuf
        let pb_request = proto::pulumist::PulumiRequest {
            working_dir: request.project.clone(),
            stack_name: request.stack.clone(),
            project_name: request.project.clone(),
            resources: request.resources.into_iter().map(|r| {
                proto::pulumist::Resource {
                    r#type: r.resource_type,
                    name: r.name,
                    properties: self.json_to_pb_map(&r.properties),
                    depends_on: r.options.as_ref()
                        .and_then(|o| o.depends_on.clone())
                        .unwrap_or_default(),
                    provider: r.options.as_ref()
                        .and_then(|o| o.provider.clone())
                        .unwrap_or_default(),
                }
            }).collect(),
            config: request.config.into_iter()
                .map(|(k, v)| (k, v.as_str().unwrap_or("").to_string()))
                .collect(),
            pulumi_config: None,
        };

        let response = Self::call_go_function_pb(PulumiDynamicGetOutputs, &pb_request)?;

        if response.success {
            // Convert outputs to JSON value
            let mut result = serde_json::Map::new();
            for output in response.outputs {
                if let Some(value) = output.value {
                    result.insert(
                        format!("{}.{}", output.resource_name, output.output_name),
                        self.pb_value_to_json(&value),
                    );
                }
            }
            Ok(Value::Object(result))
        } else {
            Err(response.error)
        }
    }

    pub fn refresh(&self, request: StackRequest) -> Result<Value, String> {
        // Convert StackRequest to protobuf
        let pb_request = proto::pulumist::PulumiRequest {
            working_dir: request.project.clone(),
            stack_name: request.stack.clone(),
            project_name: request.project.clone(),
            resources: request.resources.into_iter().map(|r| {
                proto::pulumist::Resource {
                    r#type: r.resource_type,
                    name: r.name,
                    properties: self.json_to_pb_map(&r.properties),
                    depends_on: r.options.as_ref()
                        .and_then(|o| o.depends_on.clone())
                        .unwrap_or_default(),
                    provider: r.options.as_ref()
                        .and_then(|o| o.provider.clone())
                        .unwrap_or_default(),
                }
            }).collect(),
            config: request.config.into_iter()
                .map(|(k, v)| (k, v.as_str().unwrap_or("").to_string()))
                .collect(),
            pulumi_config: None,
        };

        let response = Self::call_go_function_pb(PulumiDynamicRefresh, &pb_request)?;

        if response.success {
            // Convert outputs to JSON value
            let mut result = serde_json::Map::new();
            for output in response.outputs {
                if let Some(value) = output.value {
                    result.insert(
                        format!("{}.{}", output.resource_name, output.output_name),
                        self.pb_value_to_json(&value),
                    );
                }
            }
            Ok(Value::Object(result))
        } else {
            Err(response.error)
        }
    }

    pub fn import(&self, _request: ImportRequest) -> Result<Value, String> {
        todo!("Import functionality not yet implemented")
    }

    pub fn export_stack(&self, request: StackRequest) -> Result<Value, String> {
        // Export is the same as get_outputs
        self.get_outputs(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_dynamic_resource_creation() {
        // Example of creating an Azure resource group dynamically
        let resource = DynamicResource {
            resource_type: "azure-native:resources:ResourceGroup".to_string(),
            name: "my-resource-group".to_string(),
            properties: json!({
                "location": "eastus",
                "tags": {
                    "Environment": "Dev",
                    "Team": "Platform"
                }
            }),
            options: None,
        };

        let mut config = serde_json::Map::new();
        config.insert("azure-native:location".to_string(), json!("eastus"));

        let request = StackRequest {
            project: "test-project".to_string(),
            stack: "dev".to_string(),
            backend: Some("azblob".to_string()),
            config,
            resources: vec![resource],
        };

        // This would call the Go function in a real scenario
        println!("Request: {}", serde_json::to_string_pretty(&request).unwrap());
    }
}