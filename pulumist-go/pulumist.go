//go:build cgo

//go:generate go run codegen.go

package pulumist

/*
#include <stdlib.h>
*/
import "C"
import (
	"context"
	"fmt"
	pb "github.com/evorine/pulumist/pulumist-go/generated" // Generated protobuf types
	"github.com/pulumi/pulumi/sdk/v3/go/auto"
	"github.com/pulumi/pulumi/sdk/v3/go/pulumi"
	"google.golang.org/protobuf/proto"
	"os"
	"path/filepath"
	"unsafe"
)

// PulumiDynamicPreview performs a dry-run preview of infrastructure changes.
// This shows what resources would be created, updated, or deleted without actually making any changes.
//
// Parameters:
//   - @param request: Pointer to protobuf-encoded PulumiRequest data
//   - @param length: Length of the request data in bytes
//
// Returns a pointer to C-allocated memory containing length-prefixed PulumiResponse.
// The caller must free this memory using PulumiFree.
//
// Errors are returned in the response with success=false.
//
//export PulumiDynamicPreview
func PulumiDynamicPreview(request *C.char, length C.int) *C.char {
	return processPulumiRequest(request, length, true)
}

// PulumiDynamicDeploy performs a pulumi up equivalent action.
// It executes infrastructure deployment, creating, updating, or deleting resources as needed to match the desired state.
//
// Parameters:
//   - @param request: Pointer to protobuf-encoded PulumiRequest data
//   - @param length: Length of the request data in bytes
//
// Returns a pointer to C-allocated memory containing length-prefixed PulumiResponse.
// The caller must free this memory using PulumiFree.
//
// Errors are returned in the response with success=false.
//
//export PulumiDynamicDeploy
func PulumiDynamicDeploy(request *C.char, length C.int) *C.char {
	return processPulumiRequest(request, length, false)
}

// PulumiDynamicDestroy removes all resources from the specified stack.
// It executes pulumi destroy equivalent action, which deletes all resources in the stack.
// This is a destructive operation that cannot be undone.
//
// Parameters:
//   - @param request: Pointer to protobuf-encoded PulumiRequest data
//   - @param length: Length of the request data in bytes
//
// Returns a pointer to C-allocated memory containing length-prefixed PulumiResponse.
// The caller must free this memory using PulumiFree.
//
//export PulumiDynamicDestroy
func PulumiDynamicDestroy(requestBytes *C.char, requestLen C.int) *C.char {
	// Convert C bytes to Go bytes safely
	goBytes := C.GoBytes(unsafe.Pointer(requestBytes), requestLen)

	// Deserialize protobuf request
	var request pb.PulumiRequest
	if err := proto.Unmarshal(goBytes, &request); err != nil {
		return createFailedResponse(err)
	}

	// Create context for cancellation
	// TODO: Accept timeout from request for long-running operations
	ctx := context.Background()

	// Ensure that the working directory exists
	workDir, err := ensureWorkingDirectory(request.ProjectName)
	if err != nil {
		return createFailedResponse(fmt.Errorf("failed to ensure working directory: %w", err))
	}

	// Get existing stack
	stack, err := auto.SelectStackInlineSource(ctx, request.StackName, request.ProjectName,
		func(ctx *pulumi.Context) error { return nil },
		auto.WorkDir(workDir),
	)
	if err != nil {
		return createFailedResponse(err)
	}

	// Destroy resources
	destroyResult, err := stack.Destroy(ctx)
	if err != nil {
		return createFailedResponse(err)
	}

	var outputs []*pb.OutputItem
	outputs = append(outputs, &pb.OutputItem{
		ResourceName: "stack",
		OutputName:   "stdout",
		Value:        convertInterfaceToProtoValue(destroyResult.StdOut),
	})
	outputs = append(outputs, &pb.OutputItem{
		ResourceName: "stack",
		OutputName:   "stderr",
		Value:        convertInterfaceToProtoValue(destroyResult.StdErr),
	})
	outputs = append(outputs, &pb.OutputItem{
		ResourceName: "stack",
		OutputName:   "summary",
		Value: convertInterfaceToProtoValue(map[string]interface{}{
			"message": destroyResult.Summary.Message,
			"result":  destroyResult.Summary.Result,
		}),
	})

	return createOkResponse(outputs)
}

// PulumiDynamicGetOutputs retrieves the current outputs from a stack.
// Only explicitly exported outputs are returned, not all resource properties.
//
// Parameters:
//   - @param request: Pointer to protobuf-encoded PulumiRequest data
//   - @param length: Length of the request data in bytes
//
// Returns a pointer to C-allocated memory containing length-prefixed PulumiResponse.
// The caller must free this memory using PulumiFree.
//
//export PulumiDynamicGetOutputs
func PulumiDynamicGetOutputs(requestBytes *C.char, requestLen C.int) *C.char {
	// Convert C bytes to Go bytes safely
	goBytes := C.GoBytes(unsafe.Pointer(requestBytes), requestLen)

	// Deserialize protobuf request
	var request pb.PulumiRequest
	if err := proto.Unmarshal(goBytes, &request); err != nil {
		return createFailedResponse(err)
	}

	// Create context for cancellation
	ctx := context.Background()

	// Ensure that the working directory exists
	workDir, err := ensureWorkingDirectory(request.ProjectName)
	if err != nil {
		return createFailedResponse(fmt.Errorf("failed to ensure working directory: %w", err))
	}

	// Get existing stack
	stack, err := auto.SelectStackInlineSource(ctx, request.StackName, request.ProjectName,
		func(ctx *pulumi.Context) error { return nil },
		auto.WorkDir(workDir),
	)
	if err != nil {
		return createFailedResponse(err)
	}

	// Get outputs from the stack
	outputs, err := stack.Outputs(ctx)
	if err != nil {
		return createFailedResponse(err)
	}

	var outputItems []*pb.OutputItem
	for name, value := range outputs {
		outputItems = append(outputItems, &pb.OutputItem{
			ResourceName: "stack",
			OutputName:   name,
			Value:        convertInterfaceToProtoValue(value),
		})
	}

	return createOkResponse(outputItems)
}

func processPulumiRequest(requestBytes *C.char, requestLen C.int, isDryRun bool) *C.char {
	// Convert C bytes to Go bytes safely
	goBytes := C.GoBytes(unsafe.Pointer(requestBytes), requestLen)

	// Deserialize protobuf request
	var request pb.PulumiRequest
	if err := proto.Unmarshal(goBytes, &request); err != nil {
		return createFailedResponse(err)
	}

	// Create context for cancellation
	// TODO: Accept timeout from request for long-running operations
	ctx := context.Background()

	// Ensure that the working directory exists
	workDir, err := ensureWorkingDirectory(request.ProjectName)
	if err != nil {
		return createFailedResponse(fmt.Errorf("failed to ensure working directory: %w", err))
	}

	// Create the deployment function with dynamic resources
	// This function will be called by Pulumi's engine to define infrastructure.
	// It captures the resources from the request and registers them when executed.
	deploymentProgram := createDeploymentProgram(request.Resources)

	// Create the stack with the appropriate backend
	opts := []auto.LocalWorkspaceOption{
		auto.WorkDir(workDir),
	}

	// Always use local with passphrase
	// TODO: Support cloud based key management services (AWS KMS, Azure Key Vault etc.)
	// The passphrase is read from PULUMI_CONFIG_PASSPHRASE env var.
	opts = append(opts, auto.SecretsProvider("passphrase"))

	// Create or update the stack (as inline source)
	stack, err := auto.UpsertStackInlineSource(ctx, request.StackName, request.ProjectName, deploymentProgram, opts...)
	if err != nil {
		return createFailedResponse(err)
	}

	// Send start event
	emitEvent(&pb.Event{
		Event: &pb.Event_Prelude{
			Prelude: &pb.PreludeEvent{
				Config: make(map[string]string),
			},
		},
	})

	// Refresh first to detect drift
	emitEvent(&pb.Event{
		Event: &pb.Event_Diagnostic{
			Diagnostic: &pb.DiagnosticEvent{
				Severity: "info",
				Message:  "Refreshing stack to detect drift...",
			},
		},
	})
	refreshResult, refreshErr := stack.Refresh(ctx)
	if refreshErr != nil {
		emitEvent(&pb.Event{
			Event: &pb.Event_Diagnostic{
				Diagnostic: &pb.DiagnosticEvent{
					Severity: "warning",
					Message:  fmt.Sprintf("Refresh warning: %v", refreshErr),
				},
			},
		})
	} else {
		emitEvent(&pb.Event{
			Event: &pb.Event_Diagnostic{
				Diagnostic: &pb.DiagnosticEvent{
					Severity: "info",
					Message:  fmt.Sprintf("Refresh completed: %s", refreshResult.Summary.Message),
				},
			},
		})
	}

	// Now that we have the stack ready, we can proceed with the preview or up operation.
	if isDryRun {
		return previewStack(stack, ctx)
	} else {
		return deployStack(stack, ctx)
	}
}

// Performs a dry-run preview of the provided stack.
// This will show what changes would be made without actually applying them.
func previewStack(stack auto.Stack, ctx context.Context) *C.char {
	// Preview the stack
	preview, err := stack.Preview(ctx)

	if err != nil {
		return createFailedResponse(err)
	}

	var outputs []*pb.OutputItem
	outputs = append(outputs, &pb.OutputItem{
		ResourceName: "stack",
		OutputName:   "stdout",
		Value:        convertInterfaceToProtoValue(preview.StdOut),
	})
	outputs = append(outputs, &pb.OutputItem{
		ResourceName: "stack",
		OutputName:   "stderr",
		Value:        convertInterfaceToProtoValue(preview.StdErr),
	})
	outputs = append(outputs, &pb.OutputItem{
		ResourceName: "stack",
		OutputName:   "summary",
		Value:        convertInterfaceToProtoValue(preview.ChangeSummary),
	})

	return createOkResponse(outputs)
}

// deployStack applies the changes to the stack and returns the result.
func deployStack(stack auto.Stack, ctx context.Context) *C.char {
	// Run deployment
	upResult, err := stack.Up(ctx)

	if err != nil {
		return createFailedResponse(err)
	}

	// Send summary event
	emitEvent(&pb.Event{
		Event: &pb.Event_Summary{
			Summary: &pb.SummaryEvent{
				MayChange:       false,
				DurationSeconds: 10, // Placeholder
				ResourceChanges: map[string]int32{
					"created": 1,
				},
			},
		},
	})

	// Get outputs
	stackOutputs, err := stack.Outputs(ctx)
	if err != nil {
		return createFailedResponse(err)
	}

	var outputs []*pb.OutputItem
	outputs = append(outputs, &pb.OutputItem{
		ResourceName: "stack",
		OutputName:   "stdout",
		Value:        convertInterfaceToProtoValue(upResult.StdOut),
	})
	outputs = append(outputs, &pb.OutputItem{
		ResourceName: "stack",
		OutputName:   "stderr",
		Value:        convertInterfaceToProtoValue(upResult.StdErr),
	})
	outputs = append(outputs, &pb.OutputItem{
		ResourceName: "stack",
		OutputName:   "outputs",
		Value:        convertInterfaceToProtoValue(stackOutputs),
	})
	outputs = append(outputs, &pb.OutputItem{
		ResourceName: "stack",
		OutputName:   "summary",
		Value: convertInterfaceToProtoValue(map[string]interface{}{
			"message": upResult.Summary.Message,
			"result":  upResult.Summary.Result,
		}),
	})

	return createOkResponse(outputs)
}

func ensureWorkingDirectory(projectName string) (string, error) {
	// Create the working directory if it doesn't exist
	workDir := filepath.Join(".", projectName)
	if err := os.MkdirAll(workDir, 0755); err != nil {
		return "", fmt.Errorf("failed to create working directory: %w", err)
	}
	return workDir, nil
}

// createFailedResponse creates a PulumiResponse which represents an error and returns it as a C-compatible byte array with a length prefix.
//
// Format:
//
//	[0:4]  - Length of protobuf data (4 bytes, little-endian uint32)
//	[4:n]  - Protobuf-encoded PulumiResponse message
//
// This format is necessary because:
// 1. C strings are null-terminated, but protobuf can contain null bytes
// 2. FFI doesn't preserve array length information
//
// Memory management:
// - This function allocates memory using C.CBytes
// - The caller host MUST free this memory using PulumiFree
// - Failure to free will cause memory leaks
//
// TODO: If proto.Marshal fails, currently we ignore it. Handle this better.
func createFailedResponse(err error) *C.char {
	response := &pb.PulumiResponse{
		Success: false,
		Error:   err.Error(),
		Outputs: []*pb.OutputItem{},
	}

	// Serialize to protobuf binary format
	respBytes, _ := proto.Marshal(response) // TODO: Handle marshal error

	// Create length-prefixed response
	// Format: [4 bytes length][protobuf data]
	result := make([]byte, 4+len(respBytes))

	// Write length as 4-byte little-endian integer
	// This matches Rust's expectation for reading the length
	result[0] = byte(len(respBytes))
	result[1] = byte(len(respBytes) >> 8)
	result[2] = byte(len(respBytes) >> 16)
	result[3] = byte(len(respBytes) >> 24)

	// Copy protobuf data
	copy(result[4:], respBytes)

	// Convert to C-allocated memory that can cross the FFI boundary
	return (*C.char)(C.CBytes(result))
}

// createOkResponse creates a PulumiResponse which represents a successful process and returns it as a C-compatible byte array with a length prefix.
//
// Format:
//
//	[0:4]  - Length of protobuf data (4 bytes, little-endian uint32)
//	[4:n]  - Protobuf-encoded PulumiResponse message
//
// This format is necessary because:
// 1. C strings are null-terminated, but protobuf can contain null bytes
// 2. FFI doesn't preserve array length information
//
// Memory management:
// - This function allocates memory using C.CBytes
// - The caller host MUST free this memory using PulumiFree
// - Failure to free will cause memory leaks
//
// TODO: If proto.Marshal fails, currently we ignore it. Handle this better.
func createOkResponse(outputs []*pb.OutputItem) *C.char {
	response := &pb.PulumiResponse{
		Success: true,
		Outputs: outputs,
	}

	// Serialize to protobuf binary format
	respBytes, _ := proto.Marshal(response) // TODO: Handle marshal error

	// Create length-prefixed response
	// Format: [4 bytes length][protobuf data]
	result := make([]byte, 4+len(respBytes))

	// Write length as 4-byte little-endian integer
	// This matches Rust's expectation for reading the length
	result[0] = byte(len(respBytes))
	result[1] = byte(len(respBytes) >> 8)
	result[2] = byte(len(respBytes) >> 16)
	result[3] = byte(len(respBytes) >> 24)

	// Copy protobuf data
	copy(result[4:], respBytes)

	// Convert to C-allocated memory that can cross the FFI boundary
	return (*C.char)(C.CBytes(result))
}

// FreeAllocation frees memory allocated by Go.
//
//export FreeAllocation
func FreeAllocation(obj *C.char) {
	C.free(unsafe.Pointer(obj))
}
