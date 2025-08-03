# pulumist-go

This is an FFI-compatible Go library that provides a wrapper around Pulumi's Automation API for Go.

It has been designed to be used from `pulumist` crate developed in Rust.

## How Pulumist Communicates with Pulumi

```
┌───────────────────────┐
│   Rust Application    │
└──────────┬────────────┘
           │ FFI calls with protobuf
┌──────────▼────────────┐
│    Go FFI Layer       │ (pulumist.go + event_stream.go)
└──────────┬────────────┘
           │ Automation API calls
┌──────────▼────────────┐
│ Pulumi Automation API │ (auto.Stack, auto.LocalWorkspace)
└──────────┬────────────┘
           │ gRPC + subprocess
┌──────────▼────────────┐
│   Pulumi Engine       │ (pulumi CLI binary)
│   Language Host       │ (Go SDK runtime)
└──────────┬────────────┘
           │ gRPC
┌──────────▼────────────┐
│ Provider Plugins      │ (azure-native, aws, etc.)
└──────────┬────────────┘
           │ HTTPS REST/SDK calls
┌──────────▼────────────┐
│   Cloud APIs          │ (Azure ARM, AWS API, etc.)
└───────────────────────┘
```

## Go's Runtime Considerations

As we are using Go's FFI, we need to ensure that the Go runtime is managed properly.
I didn't do much research on this. I'm afraid of having Go's runtime hanging around after Pulumi(st) is done.

I need to investigate how to properly clean up the Go runtime after the FFI calls are done.
Or, if that's not possible, we might use a separate Go process.
