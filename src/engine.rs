use crate::error::Result;
use crate::stack::Stack;
use crate::dynamic::PulumiDynamic;

pub struct PulumiEngine {
    dynamic: PulumiDynamic,
}

impl PulumiEngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            dynamic: PulumiDynamic::new(),
        })
    }
    
    pub fn create_stack(&self, name: &str) -> StackBuilder {
        StackBuilder::new(name, &self.dynamic)
    }
}

pub struct StackBuilder<'a> {
    name: String,
    project: Option<String>,
    backend: Option<String>,
    config: serde_json::Map<String, serde_json::Value>,
    dynamic: &'a PulumiDynamic,
}

impl<'a> StackBuilder<'a> {
    fn new(name: &str, dynamic: &'a PulumiDynamic) -> Self {
        Self {
            name: name.to_string(),
            project: None,
            backend: None,
            config: serde_json::Map::new(),
            dynamic,
        }
    }
    
    pub fn with_project(mut self, project: &str) -> Self {
        self.project = Some(project.to_string());
        self
    }
    
    pub fn with_azure_backend(mut self) -> Self {
        self.backend = Some("azblob".to_string());
        self
    }
    
    pub fn with_config(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.config.insert(key.to_string(), value.into());
        self
    }
    
    pub fn build(self) -> Result<Stack> {
        Stack::new(
            self.name,
            self.project.unwrap_or_else(|| "pulumist-project".to_string()),
            self.backend,
            self.config,
            self.dynamic.clone(),
        )
    }
}