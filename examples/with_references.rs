use serde_json::json;
use std::env;
use pulumist::dynamic::DynamicResource;
use pulumist::engine::PulumiEngine;
use pulumist::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Set passphrase for local backend
    unsafe {
        env::set_var("PULUMI_CONFIG_PASSPHRASE", "testpassphrase");
    }
    
    println!("🔗 Demonstrating Output References in Pulumist\n");

    let engine = PulumiEngine::new()?;

    let stack = engine.create_stack("dev")
        .with_project("output-references-demo")
        .with_config("azure-native:location", "eastus")
        .build()?;

    println!("Creating resources with output references...\n");

    // Create a resource group
    let rg = DynamicResource {
        resource_type: "azure-native:resources:ResourceGroup".to_string(),
        name: "demo-rg".to_string(),
        properties: json!({
            "resourceGroupName": "outputrefdemo3",
            "location": "eastus",
            "tags": {
                "Purpose": "Demo output references"
            }
        }),
        options: Default::default(),
    };
    
    // Create a storage account that references the resource group name
    let storage = DynamicResource {
        resource_type: "azure-native:storage:StorageAccount".to_string(),
        name: "demo-storage".to_string(),
        properties: json!({
            "accountName": "outputrefstore5521x5",
            // This references the resource group's resourceGroupName property
            "resourceGroupName": "${demo-rg.resourceGroupName}",
            "location": "eastus",
            "sku": {
                "name": "Standard_LRS"
            },
            "kind": "StorageV2",
        }),
        options: None, // ResourceOptions would need to be defined
    };
    
    // Create a container that references the storage account
    let container = DynamicResource {
        resource_type: "azure-native:storage:BlobContainer".to_string(),
        name: "demo-container".to_string(),
        properties: json!({
            "containerName": "democontainer",
            // These reference outputs from other resources
            "resourceGroupName": "${demo-rg.resourceGroupName}",
            "accountName": "${demo-storage.accountName}",
            "publicAccess": "None",
        }),
        options: None, // ResourceOptions would need to be defined
    };

    // Preview the deployment
    println!("📋 Previewing deployment...\n");

    match stack.preview()
        .with_resource(rg.clone())
        .with_resource(storage.clone())
        .with_resource(container.clone())
        .execute()
        .await
    {
        Ok(result) => {
            println!("✅ Preview completed");
            println!("   Resources will be created with proper references");
            if let Some(summary) = result.get("summary") {
                println!("   Summary: {}", serde_json::to_string_pretty(summary)?);
            }
        }
        Err(e) => {
            println!("❌ Preview failed: {}", e);
            println!("\n💡 Note: This example requires a properly configured Pulumi environment.");
            println!("   The error above is expected if you don't have Pulumi set up.");
            println!("\n   The example demonstrates how output references would work:");
            println!("   - ${{demo-rg.resourceGroupName}} references the resource group name");
            println!("   - ${{demo-storage.accountName}} references the storage account name");
            return Err(e);
        }
    }

    // Deploy the resources
    println!("\n🚀 Deploying resources with output references...\n");

    match stack.deploy()
        .with_resource(rg)
        .with_resource(storage)
        .with_resource(container)
        .execute()
        .await
    {
        Ok(result) => {
            println!("\n✅ Deployment completed successfully!");
            println!("   Output references were resolved automatically");

            if let Some(outputs) = result.get("outputs") {
                println!("\n📤 Stack Outputs:");
                println!("{}", serde_json::to_string_pretty(outputs)?);
            }
        }
        Err(e) => {
            println!("\n❌ Deployment failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}