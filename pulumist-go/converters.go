package pulumist

import (
	"fmt"
	pb "github.com/evorine/pulumist/pulumist-go/generated"
	"github.com/pulumi/pulumi/sdk/v3/go/pulumi"
)

// convertProtoValueToInterface converts protobuf Value to Go interface{}.
func convertProtoValueToInterface(value *pb.Value) interface{} {
	if value == nil {
		return nil
	}

	switch unwrapped := value.Value.(type) {
	case *pb.Value_StringValue:
		return unwrapped.StringValue

	case *pb.Value_IntValue:
		// 64-bit integer - covers int, int32, int64 from Rust
		return unwrapped.IntValue

	case *pb.Value_DoubleValue:
		// 64-bit float - covers f32, f64 from Rust
		return unwrapped.DoubleValue

	case *pb.Value_BoolValue:
		return unwrapped.BoolValue

	case *pb.Value_ListValue:
		// Array/slice - recursively convert each element
		result := make([]interface{}, len(unwrapped.ListValue.Values))
		for i, item := range unwrapped.ListValue.Values {
			result[i] = convertProtoValueToInterface(item)
		}
		return result

	case *pb.Value_MapValue:
		// Object/map - recursively convert each field
		result := make(map[string]interface{})
		for k, v := range unwrapped.MapValue.Fields {
			result[k] = convertProtoValueToInterface(v)
		}
		return result

	case *pb.Value_BytesValue:
		// Raw bytes - for binary data
		return unwrapped.BytesValue

	default:
		// Unknown type - This is a UB and should not happen
		// TODO: Fail gracefully or log an error. What should we do here?
		return nil
	}
}

// convertInterfaceToPulumiValue converts Go values to Pulumi inputs.
//
// Pulumi's type system requires all resource properties to be Input types.
// This provides type safety and enables the engine to track dependencies.
func convertInterfaceToPulumiValue(value interface{}) pulumi.Input {
	switch wrapped := value.(type) {
	case string:
		return pulumi.String(wrapped)

	case float64:
		// JSON numbers are always float64 in Go
		return pulumi.Float64(wrapped)

	case bool:
		return pulumi.Bool(wrapped)

	case []interface{}:
		// Convert slice to Pulumi Array
		arr := pulumi.Array{}
		for _, item := range wrapped {
			// Recursively convert each element
			arr = append(arr, convertInterfaceToPulumiValue(item))
		}
		return arr

	case map[string]interface{}:
		// Convert map to Pulumi Map
		m := pulumi.Map{}
		for k, v := range wrapped {
			// Recursively convert each value
			m[k] = convertInterfaceToPulumiValue(v)
		}
		return m

	case pulumi.Input:
		// Already a Pulumi Input (e.g., from output reference)
		// Pass through unchanged
		return wrapped

	default:
		// Fallback for unknown types
		return pulumi.Any(wrapped)
	}
}

// convertInterfaceToProtoValue converts a Go interface{} to a protobuf Value message.
func convertInterfaceToProtoValue(value interface{}) *pb.Value {
	if value == nil {
		return nil
	}

	switch converted := value.(type) {
	case string:
		return &pb.Value{Value: &pb.Value_StringValue{StringValue: converted}}

	case int:
		// Convert int to int64 for protobuf
		return &pb.Value{Value: &pb.Value_IntValue{IntValue: int64(converted)}}
	case int32:
		return &pb.Value{Value: &pb.Value_IntValue{IntValue: int64(converted)}}
	case int64:
		return &pb.Value{Value: &pb.Value_IntValue{IntValue: converted}}

	case float32:
		return &pb.Value{Value: &pb.Value_DoubleValue{DoubleValue: float64(converted)}}
	case float64:
		return &pb.Value{Value: &pb.Value_DoubleValue{DoubleValue: converted}}

	case bool:
		return &pb.Value{Value: &pb.Value_BoolValue{BoolValue: converted}}

	case []interface{}:
		// Convert slice recursively
		values := make([]*pb.Value, len(converted))
		for i, item := range converted {
			values[i] = convertInterfaceToProtoValue(item)
		}
		return &pb.Value{Value: &pb.Value_ListValue{ListValue: &pb.ValueList{Values: values}}}

	case map[string]interface{}:
		// Convert map recursively
		fields := make(map[string]*pb.Value)
		for k, v := range converted {
			fields[k] = convertInterfaceToProtoValue(v)
		}
		return &pb.Value{Value: &pb.Value_MapValue{MapValue: &pb.ValueMap{Fields: fields}}}

	case []byte:
		return &pb.Value{Value: &pb.Value_BytesValue{BytesValue: converted}}

	default:
		// Fallback: convert unknown types to string
		// This handles custom types, pointers, etc.
		// WARNING: This may lose type information
		// TODO: Should we fail here instead?
		return &pb.Value{Value: &pb.Value_StringValue{StringValue: fmt.Sprintf("%value", converted)}}
	}
}
