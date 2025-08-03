use crate::error::{Result, PulumistError};
use crate::events::{DeploymentEvent, EventHandler};
use crate::dynamic::{PulumiDynamic, StackRequest, DynamicResource, ImportRequest};
use serde_json::Value;
use std::sync::Arc;
use std::thread;

pub struct Stack {
    name: String,
    project: String,
    backend: Option<String>,
    config: serde_json::Map<String, Value>,
    dynamic: PulumiDynamic,
}

impl Stack {
    pub(crate) fn new(
        name: String,
        project: String,
        backend: Option<String>,
        config: serde_json::Map<String, Value>,
        dynamic: PulumiDynamic,
    ) -> Result<Self> {
        Ok(Self {
            name,
            project,
            backend,
            config,
            dynamic,
        })
    }
    
    pub fn deploy(&self) -> DeploymentBuilder {
        DeploymentBuilder::new(self)
    }
    
    pub fn preview(&self) -> PreviewBuilder {
        PreviewBuilder::new(self)
    }
    
    pub fn destroy(&self) -> Result<()> {
        let request = StackRequest {
            project: self.project.clone(),
            stack: self.name.clone(),
            backend: self.backend.clone(),
            config: self.config.clone(),
            resources: vec![],
        };
        
        self.dynamic.destroy(request).map_err(|e| PulumistError::StackOperation(e))?;
        Ok(())
    }
    
    pub fn refresh(&self) -> RefreshBuilder {
        RefreshBuilder::new(self)
    }
    
    pub fn import(&self) -> ImportBuilder {
        ImportBuilder::new(self)
    }
    
    pub fn export(&self) -> Result<Value> {
        let request = StackRequest {
            project: self.project.clone(),
            stack: self.name.clone(),
            backend: self.backend.clone(),
            config: self.config.clone(),
            resources: vec![],
        };
        
        self.dynamic.export_stack(request).map_err(|e| PulumistError::StackOperation(e))
    }
    
    pub fn get_outputs(&self) -> Result<Value> {
        let request = StackRequest {
            project: self.project.clone(),
            stack: self.name.clone(),
            backend: self.backend.clone(),
            config: self.config.clone(),
            resources: vec![],
        };
        
        self.dynamic.get_outputs(request).map_err(|e| PulumistError::StackOperation(e))
    }
}

pub struct DeploymentBuilder<'a> {
    stack: &'a Stack,
    resources: Vec<DynamicResource>,
    event_handler: Option<Arc<dyn EventHandler>>,
}

impl<'a> DeploymentBuilder<'a> {
    fn new(stack: &'a Stack) -> Self {
        Self {
            stack,
            resources: vec![],
            event_handler: None,
        }
    }
    
    pub fn with_resource(mut self, resource: DynamicResource) -> Self {
        self.resources.push(resource);
        self
    }
    
    pub fn with_event_handler(mut self, handler: Arc<dyn EventHandler>) -> Self {
        self.event_handler = Some(handler);
        self
    }
    
    pub async fn execute(self) -> Result<Value> {
        let request = StackRequest {
            project: self.stack.project.clone(),
            stack: self.stack.name.clone(),
            backend: self.stack.backend.clone(),
            config: self.stack.config.clone(),
            resources: self.resources,
        };
        
        // If event handler is provided, set up event channel
        if let Some(handler) = self.event_handler {
            let event_receiver = crate::events::create_event_channel();
            
            // Spawn a thread to handle events
            thread::spawn(move || {
                while let Ok(event_json) = event_receiver.recv() {
                    if let Ok(event) = serde_json::from_value::<DeploymentEvent>(event_json) {
                        handler.handle_event(event);
                    }
                }
            });
        }
        
        let result = self.stack.dynamic.deploy(request)
            .map_err(|e| PulumistError::StackOperation(e));
            
        // Cleanup event channel
        crate::events::cleanup_event_channel();
        
        result
    }
}

pub struct PreviewBuilder<'a> {
    stack: &'a Stack,
    resources: Vec<DynamicResource>,
    event_handler: Option<Arc<dyn EventHandler>>,
}

impl<'a> PreviewBuilder<'a> {
    fn new(stack: &'a Stack) -> Self {
        Self {
            stack,
            resources: vec![],
            event_handler: None,
        }
    }
    
    pub fn with_resource(mut self, resource: DynamicResource) -> Self {
        self.resources.push(resource);
        self
    }
    
    pub fn with_event_handler(mut self, handler: Arc<dyn EventHandler>) -> Self {
        self.event_handler = Some(handler);
        self
    }
    
    pub async fn execute(self) -> Result<Value> {
        let request = StackRequest {
            project: self.stack.project.clone(),
            stack: self.stack.name.clone(),
            backend: self.stack.backend.clone(),
            config: self.stack.config.clone(),
            resources: self.resources,
        };
        
        // If event handler is provided, set up event channel
        if let Some(handler) = self.event_handler {
            let event_receiver = crate::events::create_event_channel();
            
            // Spawn a thread to handle events
            thread::spawn(move || {
                while let Ok(event_json) = event_receiver.recv() {
                    if let Ok(event) = serde_json::from_value::<DeploymentEvent>(event_json) {
                        handler.handle_event(event);
                    }
                }
            });
        }
        
        let result = self.stack.dynamic.preview(request)
            .map_err(|e| PulumistError::StackOperation(e));
            
        // Cleanup event channel
        crate::events::cleanup_event_channel();
        
        result
    }
}

pub struct RefreshBuilder<'a> {
    stack: &'a Stack,
    event_handler: Option<Arc<dyn EventHandler>>,
}

impl<'a> RefreshBuilder<'a> {
    fn new(stack: &'a Stack) -> Self {
        Self {
            stack,
            event_handler: None,
        }
    }
    
    pub fn with_event_handler(mut self, handler: Arc<dyn EventHandler>) -> Self {
        self.event_handler = Some(handler);
        self
    }
    
    pub async fn execute(self) -> Result<Value> {
        let request = StackRequest {
            project: self.stack.project.clone(),
            stack: self.stack.name.clone(),
            backend: self.stack.backend.clone(),
            config: self.stack.config.clone(),
            resources: vec![],
        };
        
        // If event handler is provided, set up event channel
        if let Some(handler) = self.event_handler {
            let event_receiver = crate::events::create_event_channel();
            
            // Spawn a thread to handle events
            thread::spawn(move || {
                while let Ok(event_json) = event_receiver.recv() {
                    if let Ok(event) = serde_json::from_value::<DeploymentEvent>(event_json) {
                        handler.handle_event(event);
                    }
                }
            });
        }
        
        let result = self.stack.dynamic.refresh(request)
            .map_err(|e| PulumistError::StackOperation(e));
            
        // Cleanup event channel
        crate::events::cleanup_event_channel();
        
        result
    }
}

pub struct ImportBuilder<'a> {
    stack: &'a Stack,
    resource_type: Option<String>,
    resource_name: Option<String>,
    resource_id: Option<String>,
    resources: Vec<DynamicResource>,
    event_handler: Option<Arc<dyn EventHandler>>,
}

impl<'a> ImportBuilder<'a> {
    fn new(stack: &'a Stack) -> Self {
        Self {
            stack,
            resource_type: None,
            resource_name: None,
            resource_id: None,
            resources: vec![],
            event_handler: None,
        }
    }
    
    pub fn with_resource_type(mut self, resource_type: String) -> Self {
        self.resource_type = Some(resource_type);
        self
    }
    
    pub fn with_resource_name(mut self, resource_name: String) -> Self {
        self.resource_name = Some(resource_name);
        self
    }
    
    pub fn with_resource_id(mut self, resource_id: String) -> Self {
        self.resource_id = Some(resource_id);
        self
    }
    
    pub fn with_resources(mut self, resources: Vec<DynamicResource>) -> Self {
        self.resources = resources;
        self
    }
    
    pub fn with_event_handler(mut self, handler: Arc<dyn EventHandler>) -> Self {
        self.event_handler = Some(handler);
        self
    }
    
    pub async fn execute(self) -> Result<Value> {
        let request = ImportRequest {
            project: self.stack.project.clone(),
            stack: self.stack.name.clone(),
            backend: self.stack.backend.clone(),
            resource_type: self.resource_type.ok_or_else(|| PulumistError::ConfigError("resource_type is required for import".to_string()))?,
            resource_name: self.resource_name.ok_or_else(|| PulumistError::ConfigError("resource_name is required for import".to_string()))?,
            resource_id: self.resource_id.ok_or_else(|| PulumistError::ConfigError("resource_id is required for import".to_string()))?,
            resources: self.resources,
            config: self.stack.config.clone(),
            outputs: serde_json::Map::new(),
        };
        
        // If event handler is provided, set up event channel
        if let Some(handler) = self.event_handler {
            let event_receiver = crate::events::create_event_channel();
            
            // Spawn a thread to handle events
            thread::spawn(move || {
                while let Ok(event_json) = event_receiver.recv() {
                    if let Ok(event) = serde_json::from_value::<DeploymentEvent>(event_json) {
                        handler.handle_event(event);
                    }
                }
            });
        }
        
        let result = self.stack.dynamic.import(request)
            .map_err(|e| PulumistError::StackOperation(e));
            
        // Cleanup event channel
        crate::events::cleanup_event_channel();
        
        result
    }
}