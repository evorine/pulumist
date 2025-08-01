# Pulumist

Pulumist is a Rust library that provides Foreign Function Interface (FFI) bindings to Pulumi through Go, allowing you to manage infrastructure using Rust without needing to generate types for every resource.

Pulumist doesn't need to generate types for every provider, as it uses dynamic resources. This means you can work with any Pulumi resource without needing to define a specific type for it in Rust.

> The name "Pulumist" is a portmanteau of "Pulumi" and "Mistwrite". Mistwrite was our in-house attempt to provide a infrastructure-as-code library in Rust for our closed-source new project, but development of cloud providers was slow and cumbersome therefore, we decided to use Pulumi as a backend and provide a Rust interface to it. We wanted to keep the name Mistwrite live. In a near future, we plan to opensource Mistwrite as well.

## Architecture

Pulumist provides a high-level interface to Pulumi's Automation API through a FFI bridge to Go. The architecture consists of several layers:

```
┌──────────────────────────────────────────────────────────┐
│                     User Application                     │
├──────────────────────────────────────────────────────────┤
│                   pulumist-core (Rust)                   │
│  • PulumiEngine    • Stack         • Event Handlers      │
│  • Builders        • Error Types   • Resource Types      │
├──────────────────────────────────────────────────────────┤
│                   pulumist-ffi (Rust)                    │
│  • FFI Bindings    • Type Marshaling                     │
│  • Event Channel   • Safety Wrappers                     │
├──────────────────────────────────────────────────────────┤
│                   CGO Boundary                           │
├──────────────────────────────────────────────────────────┤
│                   pulumist-go (Go)                       │
│  • Pulumi SDK Wrapper  • Dynamic Resources               │
│  • Event Callbacks     • Output Resolution               │
├──────────────────────────────────────────────────────────┤
│              Pulumi Automation API (Go SDK)              │
└──────────────────────────────────────────────────────────┘
```

Its core components:

- **pulumist-core**: The high-level Rust API that applications interact with.
- **pulumist-ffi**: The FFI layer that bridges Rust and Go.
- **pulumist-go**: The Go library that wraps Pulumi's Automation API and provides dynamic resource management.

### FFI Bridge Design

Ownership rules:
- Go allocates memory for return values
- Rust is responsible for freeing Go-allocated memory
- Rust allocates memory for parameters passed to Go
- Go must not store pointers to Rust-allocated memory

The data is serialized using protobuf.
