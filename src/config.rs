//! Simplified configuration API for Pulumi operations
//!
//! This module provides a more ergonomic way to configure Pulumi
//! without dealing with protobuf types directly.

use std::collections::HashMap;

/// Configuration for Pulumi operations
#[derive(Debug, Clone, Default)]
pub struct PulumiConfig {
    /// Secrets provider configuration
    pub secrets: SecretsConfig,
    /// Backend storage configuration  
    pub backend: BackendConfig,
    /// Runtime options
    pub runtime: RuntimeOptions,
    /// Environment variable overrides
    pub environment: HashMap<String, String>,
    /// Custom Pulumi home directory
    pub pulumi_home: Option<String>,
    /// Log level (debug, info, warn, error)
    pub log_level: Option<String>,
}

/// Secrets provider configuration
#[derive(Debug, Clone)]
pub enum SecretsConfig {
    /// Passphrase-based encryption (default)
    Passphrase(String),
    /// AWS KMS encryption
    AwsKms {
        key_id: String,
        region: Option<String>,
        access_key_id: Option<String>,
        secret_access_key: Option<String>,
    },
    /// Azure Key Vault encryption
    AzureKeyVault {
        key_url: String,
        client_id: Option<String>,
        client_secret: Option<String>,
        tenant_id: Option<String>,
    },
    /// Google Cloud KMS encryption
    GcpKms {
        key_name: String,
        credentials_json: Option<String>,
    },
    /// No encryption (development only)
    None,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        // Default to environment variable
        SecretsConfig::Passphrase(String::new())
    }
}

/// Backend storage configuration
#[derive(Debug, Clone)]
pub enum BackendConfig {
    /// Local file storage (default)
    Local {
        path: Option<String>,
    },
    /// S3 backend
    S3 {
        bucket: String,
        region: String,
        access_key_id: Option<String>,
        secret_access_key: Option<String>,
        endpoint: Option<String>,
    },
    /// Azure Blob Storage backend
    AzureBlob {
        storage_account: String,
        container: String,
        access_key: Option<String>,
        sas_token: Option<String>,
    },
    /// Pulumi Service backend
    PulumiService {
        url: String,
        access_token: String,
    },
}

impl Default for BackendConfig {
    fn default() -> Self {
        BackendConfig::Local { path: None }
    }
}

/// Runtime options for Pulumi operations
#[derive(Debug, Clone, Default)]
pub struct RuntimeOptions {
    /// Number of resources to process in parallel
    pub parallel: Option<u32>,
    /// Operation timeout in seconds
    pub timeout_seconds: Option<u32>,
    /// Show configuration values
    pub show_config: bool,
    /// Show detailed replacement steps
    pub show_replacement_steps: bool,
    /// Show stack outputs after operation
    pub show_stack_outputs: bool,
    /// Disable state checkpoints (dangerous!)
    pub disable_checkpoints: bool,
    /// Hide sensitive outputs
    pub suppress_outputs: bool,
}

impl PulumiConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for the configuration
    pub fn builder() -> PulumiConfigBuilder {
        PulumiConfigBuilder::default()
    }

    /// Set passphrase directly (instead of using environment variable)
    pub fn with_passphrase(mut self, passphrase: impl Into<String>) -> Self {
        self.secrets = SecretsConfig::Passphrase(passphrase.into());
        self
    }

    /// Use AWS KMS for secrets
    pub fn with_aws_kms(mut self, key_id: impl Into<String>) -> Self {
        self.secrets = SecretsConfig::AwsKms {
            key_id: key_id.into(),
            region: None,
            access_key_id: None,
            secret_access_key: None,
        };
        self
    }

    /// Use S3 backend
    pub fn with_s3_backend(mut self, bucket: impl Into<String>, region: impl Into<String>) -> Self {
        self.backend = BackendConfig::S3 {
            bucket: bucket.into(),
            region: region.into(),
            access_key_id: None,
            secret_access_key: None,
            endpoint: None,
        };
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Set parallelism
    pub fn with_parallel(mut self, parallel: u32) -> Self {
        self.runtime.parallel = Some(parallel);
        self
    }

    /// Set timeout
    pub fn with_timeout_seconds(mut self, seconds: u32) -> Self {
        self.runtime.timeout_seconds = Some(seconds);
        self
    }
}

/// Builder for PulumiConfig
#[derive(Debug, Default)]
pub struct PulumiConfigBuilder {
    config: PulumiConfig,
}

impl PulumiConfigBuilder {
    /// Set the secrets provider
    pub fn secrets(mut self, secrets: SecretsConfig) -> Self {
        self.config.secrets = secrets;
        self
    }

    /// Set passphrase directly
    pub fn passphrase(mut self, passphrase: impl Into<String>) -> Self {
        self.config.secrets = SecretsConfig::Passphrase(passphrase.into());
        self
    }

    /// Use AWS KMS
    pub fn aws_kms(mut self, key_id: impl Into<String>) -> Self {
        self.config.secrets = SecretsConfig::AwsKms {
            key_id: key_id.into(),
            region: None,
            access_key_id: None,
            secret_access_key: None,
        };
        self
    }

    /// Set the backend
    pub fn backend(mut self, backend: BackendConfig) -> Self {
        self.config.backend = backend;
        self
    }

    /// Use local backend
    pub fn local_backend(mut self, path: Option<String>) -> Self {
        self.config.backend = BackendConfig::Local { path };
        self
    }

    /// Use S3 backend
    pub fn s3_backend(mut self, bucket: impl Into<String>, region: impl Into<String>) -> Self {
        self.config.backend = BackendConfig::S3 {
            bucket: bucket.into(),
            region: region.into(),
            access_key_id: None,
            secret_access_key: None,
            endpoint: None,
        };
        self
    }

    /// Set runtime options
    pub fn runtime(mut self, runtime: RuntimeOptions) -> Self {
        self.config.runtime = runtime;
        self
    }

    /// Set parallelism
    pub fn parallel(mut self, parallel: u32) -> Self {
        self.config.runtime.parallel = Some(parallel);
        self
    }

    /// Set timeout
    pub fn timeout_seconds(mut self, seconds: u32) -> Self {
        self.config.runtime.timeout_seconds = Some(seconds);
        self
    }

    /// Add environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.environment.insert(key.into(), value.into());
        self
    }

    /// Set Pulumi home directory
    pub fn pulumi_home(mut self, path: impl Into<String>) -> Self {
        self.config.pulumi_home = Some(path.into());
        self
    }

    /// Set log level
    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.config.log_level = Some(level.into());
        self
    }

    /// Build the configuration
    pub fn build(self) -> PulumiConfig {
        self.config
    }
}

/// Extension methods for converting to protobuf
impl PulumiConfig {
    #[doc(hidden)]
    pub fn to_protobuf(&self) -> Option<crate::proto::pulumist::PulumiConfiguration> {
        use crate::proto::pulumist::{
            PulumiConfiguration, SecretsProvider, PassphraseProvider, CloudKmsProvider,
            LocalProvider, BackendConfig as PbBackendConfig, LocalBackend, S3Backend,
            AzureBlobBackend, CloudBackend,
            secrets_provider, backend_config,
        };

        let secrets_provider = match &self.secrets {
            SecretsConfig::Passphrase(passphrase) => {
                if passphrase.is_empty() {
                    None // Use environment variable
                } else {
                    Some(SecretsProvider {
                        provider: Some(secrets_provider::Provider::Passphrase(PassphraseProvider {
                            passphrase: passphrase.clone(),
                        })),
                    })
                }
            }
            SecretsConfig::AwsKms { key_id, region, access_key_id, secret_access_key } => {
                let mut credentials = HashMap::new();
                if let Some(region) = region {
                    credentials.insert("AWS_REGION".to_string(), region.clone());
                }
                if let Some(key) = access_key_id {
                    credentials.insert("AWS_ACCESS_KEY_ID".to_string(), key.clone());
                }
                if let Some(secret) = secret_access_key {
                    credentials.insert("AWS_SECRET_ACCESS_KEY".to_string(), secret.clone());
                }
                
                Some(SecretsProvider {
                    provider: Some(secrets_provider::Provider::CloudKms(CloudKmsProvider {
                        provider_type: "awskms".to_string(),
                        key_id: key_id.clone(),
                        credentials,
                    })),
                })
            }
            SecretsConfig::AzureKeyVault { key_url, client_id, client_secret, tenant_id } => {
                let mut credentials = HashMap::new();
                if let Some(id) = client_id {
                    credentials.insert("AZURE_CLIENT_ID".to_string(), id.clone());
                }
                if let Some(secret) = client_secret {
                    credentials.insert("AZURE_CLIENT_SECRET".to_string(), secret.clone());
                }
                if let Some(tenant) = tenant_id {
                    credentials.insert("AZURE_TENANT_ID".to_string(), tenant.clone());
                }
                
                Some(SecretsProvider {
                    provider: Some(secrets_provider::Provider::CloudKms(CloudKmsProvider {
                        provider_type: "azurekeyvault".to_string(),
                        key_id: key_url.clone(),
                        credentials,
                    })),
                })
            }
            SecretsConfig::GcpKms { key_name, credentials_json } => {
                let mut credentials = HashMap::new();
                if let Some(creds) = credentials_json {
                    credentials.insert("GOOGLE_CREDENTIALS".to_string(), creds.clone());
                }
                
                Some(SecretsProvider {
                    provider: Some(secrets_provider::Provider::CloudKms(CloudKmsProvider {
                        provider_type: "gcpkms".to_string(),
                        key_id: key_name.clone(),
                        credentials,
                    })),
                })
            }
            SecretsConfig::None => {
                Some(SecretsProvider {
                    provider: Some(secrets_provider::Provider::Local(LocalProvider {})),
                })
            }
        };

        let backend = match &self.backend {
            BackendConfig::Local { path } => {
                path.as_ref().map(|p| PbBackendConfig {
                    backend: Some(backend_config::Backend::Local(LocalBackend {
                        path: p.clone(),
                    })),
                })
            }
            BackendConfig::S3 { bucket, region, access_key_id, secret_access_key, .. } => {
                Some(PbBackendConfig {
                    backend: Some(backend_config::Backend::S3(S3Backend {
                        bucket: bucket.clone(),
                        region: region.clone(),
                        access_key: access_key_id.clone().unwrap_or_default(),
                        secret_key: secret_access_key.clone().unwrap_or_default(),
                        session_token: String::new(),
                    })),
                })
            }
            BackendConfig::AzureBlob { storage_account, container, access_key, sas_token } => {
                Some(PbBackendConfig {
                    backend: Some(backend_config::Backend::AzureBlob(AzureBlobBackend {
                        storage_account: storage_account.clone(),
                        container: container.clone(),
                        access_key: access_key.clone().unwrap_or_default(),
                        sas_token: sas_token.clone().unwrap_or_default(),
                    })),
                })
            }
            BackendConfig::PulumiService { url, access_token } => {
                Some(PbBackendConfig {
                    backend: Some(backend_config::Backend::Cloud(CloudBackend {
                        url: url.clone(),
                        api_token: access_token.clone(),
                    })),
                })
            }
        };

        Some(PulumiConfiguration {
            secrets_provider,
            backend,
            environment: self.environment.clone(),
            pulumi_home: self.pulumi_home.clone().unwrap_or_default(),
            log_level: self.log_level.clone().unwrap_or_default(),
        })
    }
}