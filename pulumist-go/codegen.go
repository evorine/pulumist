//go:build ignore

package main

import (
	"go/build"
	"log"
	"os"
	"os/exec"
)

func main() {
	tools := []string{
		"google.golang.org/protobuf/cmd/protoc-gen-go@latest",
		"google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest",
	}

	// Install the required tools
	for _, tool := range tools {
		log.Printf("Installing %s...", tool)
		cmd := exec.Command("go", "install", tool)
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		if err := cmd.Run(); err != nil {
			log.Fatalf("Failed to install %s: %v", tool, err)
		}
	}

	// Generate the protobuf code
	gopath := build.Default.GOPATH
	os.MkdirAll("./generated/", os.ModePerm)
	cmd := exec.Command("protoc", "--proto_path=../proto", "--go_out=./generated/", "--go_opt=paths=source_relative", "pulumist.proto")
	// Add "$GOPATH/bin" to the PATH
	cmd.Env = append(os.Environ(), "PATH="+gopath+"/bin:"+os.Getenv("PATH"))
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		log.Fatalf("Failed to generate protobuf code: %v", err)
	}
	log.Println("Protobuf code generated successfully!")
}
