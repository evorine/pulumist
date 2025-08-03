use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=pulumist-go/");
    println!("cargo:rerun-if-changed=proto/");
    
    // Generate protobuf code
    prost_build::compile_protos(&["proto/pulumist.proto"], &["proto/"])
        .expect("Failed to compile protobuf");
    
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let go_dir = PathBuf::from("pulumist-go");
    
    // Build the Go library as a static library
    let output = Command::new("go")
        .current_dir(&go_dir)
        .args(&[
            "build",
            "-buildmode=c-archive",
            "-o",
            out_dir.join("libpulumist.a").to_str().unwrap(),
            ".",
        ])
        .output()
        .expect("Failed to build Go library");
    
    if !output.status.success() {
        panic!(
            "Go build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    
    // Link the static library
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=pulumist");
    
    // Platform-specific linking
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    } else if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=pthread");
        println!("cargo:rustc-link-lib=dylib=dl");
    } else if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=dylib=ws2_32");
        println!("cargo:rustc-link-lib=dylib=userenv");
        println!("cargo:rustc-link-lib=dylib=kernel32");
        println!("cargo:rustc-link-lib=dylib=ntdll");
    }
}