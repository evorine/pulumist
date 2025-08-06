package main

import (
	"fmt"
	pb "github.com/evorine/pulumist/pulumist-go/generated"
	"github.com/pulumi/pulumi/sdk/v3/go/pulumi"
	"regexp"
	"strings"
)

// createDeploymentProgram creates a Pulumi program from dynamic resources
func createDeploymentProgram(resources []*pb.Resource) pulumi.RunFunc {
	return func(ctx *pulumi.Context) error {
		resourceMap := make(map[string]pulumi.Resource)
		resourceOutputs := make(map[string]pulumi.Output)

		for _, res := range resources {
			// Send pre-create event
			emitEvent(&pb.Event{
				Event: &pb.Event_ResourcePre{
					ResourcePre: &pb.ResourcePreEvent{
						Metadata: &pb.ResourceMetadata{
							Op:   "create",
							Urn:  fmt.Sprintf("urn:pulumi::%s::%s::%s::%s", ctx.Stack(), ctx.Project(), res.Type, res.Name),
							Type: res.Type,
							New:  true,
						},
						Planning: false,
					},
				},
			})

			// Step 1: Convert protobuf properties to Go types
			properties := make(map[string]interface{})
			for k, v := range res.Properties {
				properties[k] = convertProtoValueToInterface(v)
			}

			// Step 2: Resolve ${resource.output} references
			resolvedProperties := resolveReferences(properties, resourceOutputs)

			// Step 3: Convert to Pulumi inputs
			// Everything passed to RegisterResource must be a Pulumi Input type
			// This wraps plain values and preserves existing Outputs
			inputs := pulumi.Map{}
			for k, v := range resolvedProperties {
				inputs[k] = convertInterfaceToPulumiValue(v)
			}

			// Step 4: Build resource options
			// These control resource behavior like dependencies, deletion policy, etc.
			var opts []pulumi.ResourceOption

			// Handle explicit dependencies
			// Note: Implicit dependencies from output references are handled automatically
			if len(res.DependsOn) > 0 {
				var deps []pulumi.Resource
				for _, depName := range res.DependsOn {
					if depRes, ok := resourceMap[depName]; ok {
						deps = append(deps, depRes)
					} else {
						// Dependency not found.
						// TODO: Send diagnostic event
						fmt.Printf("Warning: Dependency %s not found for resource %s\n", depName, res.Name)
					}
				}
				if len(deps) > 0 {
					opts = append(opts, pulumi.DependsOn(deps))
				}
			}

			// Handle custom provider
			if res.Provider != "" {
				// TODO: Implement provider resolution
			}

			// Step 5: Register the resource with Pulumi
			// This is where the magic happens - Pulumi will:
			// 1. Validate the resource type exists
			// 2. Find the appropriate provider plugin
			// 3. Send the inputs to the provider
			// 4. Provider creates/updates the actual cloud resource
			// 5. Return the resource state and outputs
			var resource pulumi.CustomResourceState
			err := ctx.RegisterResource(
				res.Type,
				res.Name,
				inputs,
				&resource, // Will be populated with resource state
				opts...,
			)
			if err != nil {
				// Send failure event
				emitEvent(&pb.Event{
					Event: &pb.Event_ResourceFailed{
						ResourceFailed: &pb.ResOpFailedEvent{
							Metadata: &pb.ResourceMetadata{
								Op:   "create",
								Urn:  fmt.Sprintf("urn:pulumi::%s::%s::%s::%s", ctx.Stack(), ctx.Project(), res.Type, res.Name),
								Type: res.Type,
								New:  true,
							},
							Status: 1,
							Steps:  0,
						},
					},
				})
				return err
			}

			// Send success event
			emitEvent(&pb.Event{
				Event: &pb.Event_ResourceOutputs{
					ResourceOutputs: &pb.ResOutputsEvent{
						Metadata: &pb.ResourceMetadata{
							Op:   "create",
							Urn:  fmt.Sprintf("urn:pulumi::%s::%s::%s::%s", ctx.Stack(), ctx.Project(), res.Type, res.Name),
							Type: res.Type,
							New:  true,
						},
						Planning: false,
					},
				},
			})

			// Store reference for dependencies
			resourceMap[res.Name] = &resource

			// Step 6: Store outputs for reference resolution
			// This is critical for ${resource.property} syntax to work.
			//
			// Properties storage strategy:
			// 1. Store known outputs by name (resource.id, resource.name, etc.)
			// 2. Create a map containing all properties
			// 3. Handle provider-specific outputs

			// First, store the universal ID that all resources have
			// Every Pulumi resource has an ID after creation
			resourceOutputs[res.Name+".id"] = resource.ID()

			// Create a special output that contains all the resource's properties
			// This allows referencing any property like ${resource.propertyName}
			//
			// LIMITATION: Currently we're storing input properties as outputs.
			// Ideally, we would:
			// 1. Get actual outputs from the provider after creation
			// 2. Store computed properties (like endpoints, URLs)
			// 3. Handle properties that change during creation
			//
			// TODO: Use provider SDK to get real outputs
			resourceAllOutputs := pulumi.All(resource.ID()).ApplyT(func(args []interface{}) (map[string]interface{}, error) {
				// Return all the inputs as outputs so they can be referenced
				outputMap := make(map[string]interface{})

				// Add all input properties as potential outputs
				// WARNING: These are inputs, not actual outputs from the provider
				for key, value := range res.Properties {
					outputMap[key] = convertProtoValueToInterface(value)
				}

				// Add the ID as a standard output
				outputMap["id"] = args[0]

				return outputMap, nil
			}).(pulumi.MapOutput)

			// Step 6: Store outputs for reference resolution
			// This is critical for ${resource.output} syntax to work

			/* HANDLED DIFFERENTLY MAYBE? THE CODE ABOVE HANDLES THIS. TEST BEFORE REMOVING.

			// Special case: Azure Resource Groups
			// Resource groups are special because other Azure resources need
			// the resource group name, but it's an input, not an output.
			// We make it available as an output for convenience.
			if res.Type == "azure-native:resources:ResourceGroup" {
				if nameVal, ok := res.Properties["resourceGroupName"]; ok {
					// Store as output so ${rg.resourceGroupName} works
					resourceOutputs[res.Name+".resourceGroupName"] = pulumi.ToOutput(convertProtoValueToInterface(nameVal))
				}
				// Also store the name property for consistency
				resourceOutputs[res.Name+".name"] = pulumi.String(res.Name).ToStringOutput()
			} else {
				// For other resources, make all input properties available as outputs
				// This is a convenience feature - in standard Pulumi, you'd need
				// to explicitly export outputs. Here we make everything referenceable.
				for key, value := range res.Properties {
					outputKey := res.Name + "." + key
					resourceOutputs[outputKey] = pulumi.ToOutput(convertProtoValueToInterface(value))
				}
			}
			*/

			// Store the entire resource output map for complex references
			resourceOutputs[res.Name] = resourceAllOutputs
		}

		// TODO: Step 7: Export stack outputs
		return nil
	}
}

// resolveReferences resolves ${resource.property} references in properties.
// This enables dynamic references between resources using interpolation syntax.
//
// How it works:
// 1. Scans all property values for ${...} patterns
// 2. Extracts resource name and output name from the pattern
// 3. Looks up the corresponding Pulumi Output in resourceOutputs map
// 4. Replaces the string reference with the actual Output object
//
// Example transformations:
//
//	"${my-rg.id}" -> resourceOutputs["my-rg.id"] (Pulumi Output)
//	"${storage.endpoint.fqdn}" -> Nexted references are undefined behavior for now.
//	"normal string" -> "normal string" (unchanged)
//
// This allows the host to specify dependencies without knowing Go types:
//
// TODO: Add cycle detection to prevent infinite loops
// TODO: Support nested object references like ${resource.output.field}
// TODO: Support a proper string interpolation syntax: "${r1.host}:${r1.port}"
func resolveReferences(properties map[string]interface{}, resourceOutputs map[string]pulumi.Output) map[string]interface{} {
	resolved := make(map[string]interface{})

	// Process each property recursively
	for key, value := range properties {
		resolved[key] = resolveValue(value, resourceOutputs)
	}

	return resolved
}

// resolveValue recursively resolves output references in a value
func resolveValue(value interface{}, resourceOutputs map[string]pulumi.Output) interface{} {
	switch v := value.(type) {
	case string:
		// Look for ${resource.property} pattern
		re := regexp.MustCompile(`\$\{([^.]+)\.([^}]+)\}`)
		matches := re.FindAllStringSubmatch(v, -1)

		if len(matches) == 0 {
			return v
		}

		// Process references found in the string
		// Currently only supports full string replacement (not partial)
		for _, match := range matches {
			resourceName := match[1] // e.g., "my-rg"
			propertyPath := match[2] // e.g., "name" or "properties.id"

			// Strategy 1: Check for direct output (most common case)
			// This handles outputs we explicitly stored like "my-rg.id"
			outputKey := resourceName + "." + propertyPath
			if output, exists := resourceOutputs[outputKey]; exists {
				// Found it! Return the Pulumi Output directly
				// This preserves the Output type for dependency tracking
				return output
			}

			// Strategy 2: Handle nested property paths
			// Example: ${storage.properties.endpoints.blob}
			pathParts := strings.Split(propertyPath, ".")
			if len(pathParts) > 1 {
				// Try to get the root property first
				rootKey := resourceName + "." + pathParts[0]
				if rootOutput, exists := resourceOutputs[rootKey]; exists {
					// TODO: Apply transformation to extract nested value
					// This requires using Pulumi's Apply functions
					return rootOutput
				}
			}

			// Strategy 3: Check if we have the entire resource as output
			// This happens when we store all resource properties as a map
			if resourceOutput, exists := resourceOutputs[resourceName]; exists {
				// Extract nested property using Apply
				// This creates a new Output that depends on the resource
				return resourceOutput.(pulumi.MapOutput).ApplyT(func(m map[string]interface{}) interface{} {
					// Navigate to the nested property
					return getNestedValue(m, propertyPath)
				})
			}

			// Not found - this reference doesn't exist
			// TODO: Send diagnostic event warning about unknown reference
			// For now, leave the placeholder as-is
		}

		return v

	case map[string]interface{}:
		resolved := make(map[string]interface{})
		for k, v := range v {
			resolved[k] = resolveValue(v, resourceOutputs)
		}
		return resolved

	case []interface{}:
		resolved := make([]interface{}, len(v))
		for i, item := range v {
			resolved[i] = resolveValue(item, resourceOutputs)
		}
		return resolved

	default:
		return v
	}
}

// getNestedValue extracts a nested value from a map using dot notation.
// This supports accessing nested properties in complex objects.
//
// Examples:
//   - "name" -> object["name"]
//   - "address.city" -> object["address"]["city"]
//   - "config.database.host" -> object["config"]["database"]["host"]
//
// Returns the value at the path, or nil if not found.
//
// TODO: Currently, we only support single-level: ${resource.output}
func getNestedValue(object map[string]interface{}, path string) interface{} {
	// Split the path into components
	parts := strings.Split(path, ".")
	current := object

	// Navigate through nested maps
	for i, part := range parts {
		if val, ok := current[part]; ok {
			if i == len(parts)-1 {
				// Found the target value
				return val
			}
			// Not the last part, try to navigate deeper
			if nextMap, ok := val.(map[string]interface{}); ok {
				current = nextMap
			} else {
				// Value exists but is not a map, can't navigate deeper
				return nil
			}
		} else {
			// Property not found at this level
			return nil
		}
	}

	return nil
}
