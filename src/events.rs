use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

lazy_static::lazy_static! {
    static ref EVENT_SENDER: Mutex<Option<Sender<Value>>> = Mutex::new(None);
}

/// FFI callback function that receives events from Go
pub unsafe extern "C" fn event_callback(event_json: *const c_char) {
    if event_json.is_null() {
        return;
    }

    let event_str: &str;
    unsafe {
        event_str = match CStr::from_ptr(event_json).to_str() {
            Ok(s) => s,
            Err(_) => return,
        };
    }

    let event_value: Value = match serde_json::from_str(event_str) {
        Ok(v) => v,
        Err(_) => return,
    };

    // Send event through channel if available
    if let Ok(sender_guard) = EVENT_SENDER.lock() {
        if let Some(sender) = &*sender_guard {
            let _ = sender.send(event_value);
        }
    }
}

/// Creates an event channel and registers the callback
pub fn create_event_channel() -> Receiver<Value> {
    let (sender, receiver) = channel();
    
    // Store the sender
    if let Ok(mut sender_guard) = EVENT_SENDER.lock() {
        *sender_guard = Some(sender);
    }
    
    // Register the callback with Go
    unsafe {
        super::RegisterEventCallback(Some(event_callback));
    }
    
    receiver
}

/// Unregisters the event callback
pub fn cleanup_event_channel() {
    // Clear the sender
    if let Ok(mut sender_guard) = EVENT_SENDER.lock() {
        *sender_guard = None;
    }
    
    // Unregister the callback
    unsafe {
        super::UnregisterEventCallback();
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DeploymentEvent {
    #[serde(rename = "preludeEvent")]
    Prelude {
        message: String,
    },

    #[serde(rename = "resourcePreEvent")]
    ResourcePre {
        resource: ResourceEvent,
        metadata: EventMetadata,
    },

    #[serde(rename = "resourceOutputsEvent")]
    ResourceOutputs {
        resource: ResourceEvent,
        metadata: EventMetadata,
    },

    #[serde(rename = "resourceOperationFailedEvent")]
    ResourceOperationFailed {
        resource: ResourceEvent,
        status: ResourceStatus,
        steps: i32,
        metadata: EventMetadata,
    },

    #[serde(rename = "diagnosticEvent")]
    Diagnostic {
        severity: DiagnosticSeverity,
        message: String,
        resource: Option<ResourceEvent>,
    },

    #[serde(rename = "summaryEvent")]
    Summary {
        message: String,
        duration_seconds: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEvent {
    pub urn: String,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub name: String,
    pub operation: ResourceOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceOperation {
    Create,
    Update,
    Delete,
    Replace,
    CreateReplacement,
    DeleteReplaced,
    Read,
    Import,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceStatus {
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticSeverity {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub duration_seconds: Option<f64>,
    pub progress: Option<Progress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub current: i32,
    pub total: i32,
}

/// Trait for handling deployment events
pub trait EventHandler: Send + Sync {
    fn handle_event(&self, event: DeploymentEvent);
}

/// Simple event handler that prints to stdout
pub struct PrintEventHandler;

impl PrintEventHandler {
    pub fn new() -> Self {
        PrintEventHandler
    }
}

impl EventHandler for PrintEventHandler {
    fn handle_event(&self, event: DeploymentEvent) {
        match event {
            DeploymentEvent::Prelude { message } => {
                println!("🚀 {}", message);
            }
            DeploymentEvent::ResourcePre { resource, metadata } => {
                let op = match resource.operation {
                    ResourceOperation::Create => "Creating",
                    ResourceOperation::Update => "Updating",
                    ResourceOperation::Delete => "Deleting",
                    ResourceOperation::Replace => "Replacing",
                    _ => "Processing",
                };

                if let Some(progress) = metadata.progress {
                    println!("[{}/{}] {} {} ({})",
                             progress.current,
                             progress.total,
                             op,
                             resource.resource_type,
                             resource.name
                    );
                } else {
                    println!("{} {} ({})", op, resource.resource_type, resource.name);
                }
            }
            DeploymentEvent::ResourceOutputs { resource, metadata } => {
                let duration = metadata.duration_seconds.unwrap_or(0.0);
                println!("✅ {} {} created ({:.1}s)",
                         resource.resource_type,
                         resource.name,
                         duration
                );
            }
            DeploymentEvent::ResourceOperationFailed { resource, .. } => {
                println!("❌ Failed to {} {} ({})",
                         match resource.operation {
                             ResourceOperation::Create => "create",
                             ResourceOperation::Update => "update",
                             ResourceOperation::Delete => "delete",
                             _ => "process",
                         },
                         resource.resource_type,
                         resource.name
                );
            }
            DeploymentEvent::Diagnostic { severity, message, .. } => {
                let prefix = match severity {
                    DiagnosticSeverity::Error => "❌ ERROR",
                    DiagnosticSeverity::Warning => "⚠️  WARN",
                    DiagnosticSeverity::Info => "ℹ️  INFO",
                    DiagnosticSeverity::Debug => "🔍 DEBUG",
                };
                println!("{}: {}", prefix, message);
            }
            DeploymentEvent::Summary { message, duration_seconds } => {
                println!("\n📊 Summary: {} (took {:.1}s)", message, duration_seconds);
            }
        }
    }
}