package main

/*
#include <stdlib.h>
typedef void (*event_callback)(const char* event_json);
static void call_event_callback(event_callback cb, const char* event_data) {
    if (cb != NULL) {
        cb(event_data);
    }
}
*/
import "C"
import (
	pb "github.com/evorine/pulumist/pulumist-go/generated"
	"google.golang.org/protobuf/proto"
	"unsafe"
)

var (
	// A global variable to store the callback function for sending events back to the host application.
	// WARNING: This is not thread-safe and will cause data races if accessed concurrently.
	// TODO: Improve this currentEventCallback global variable to be thread-safe.
	currentEventCallback C.event_callback
)

// RegisterEventCallback registers a C function to receive event notifications during Pulumi operations.
// Only one callback can be registered at a time.
//
// callback should be a C function pointer with signature void (*)(const char*). Pass NULL to clear the callback.
//
// The callback will receive length-prefixed protobuf-encoded Event messages.
//
// Thread safety:
//   - // TODO: NOT thread-safe. We should protect with mutex.
//   - Callback may be invoked from different goroutines
//
// Memory management:
//   - Event data is freed by the sender after callback returns
//
//export RegisterEventCallback
func RegisterEventCallback(callback C.event_callback) {
	currentEventCallback = callback
}

// UnregisterEventCallback clears the currently registered event callback.
//
//export UnregisterEventCallback
func UnregisterEventCallback() {
	currentEventCallback = nil
}

// sendEvent serializes an event to protobuf and sends it to the registered host callback.
// This enables real-time streaming of operation progress back to the host application.
//
// Safety Notes:
// - Accesses global currentEventCallback without synchronization (race condition)
// - C.CString allocates memory that is freed by C runtime after callback returns
// - If callback panics/throws in host, it could corrupt memory. But whatever, if the host panics, we can't recover anyway.
func emitEvent(event *pb.Event) {
	// Event callback registration is optional.
	if currentEventCallback == nil {
		return
	}

	// Serialize event to protobuf
	eventBytes, err := proto.Marshal(event)
	// If serialization fails, currently we don't have a way to report this back to the host.
	if err != nil {
		// TODO: Log error or send error event
		return
	}

	// Create length-prefixed payload (same format as responses)
	result := make([]byte, 4+len(eventBytes))
	result[0] = byte(len(eventBytes))
	result[1] = byte(len(eventBytes) >> 8)
	result[2] = byte(len(eventBytes) >> 16)
	result[3] = byte(len(eventBytes) >> 24)
	copy(result[4:], eventBytes)

	// This should allocate on C heap, making it safe to pass to FFI
	cEventData := (*C.char)(C.CBytes(result))
	defer C.free(unsafe.Pointer(cEventData))

	// Call through CGo wrapper with the length
	C.call_event_callback(currentEventCallback, cEventData)
}
