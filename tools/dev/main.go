// SPDX-License-Identifier: Apache-2.0

package main

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"time"
)

const protocVersion = "libprotoc 36.2"
const generatorVersion = "protoc-gen-go v1.36.12"

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) != 1 {
		return errors.New("usage: go run ./tools/dev build|build-freebsd|fmt|checkfmt|generate|check-generated|notices|license|clean")
	}
	root, err := repositoryRoot()
	if err != nil {
		return err
	}
	if err := os.Chdir(root); err != nil {
		return err
	}
	switch args[0] {
	case "build":
		return build("", "")
	case "build-freebsd":
		for _, arch := range []string{"amd64", "arm64"} {
			if err := build("freebsd", arch); err != nil {
				return err
			}
		}
		return nil
	case "fmt", "checkfmt":
		return format(args[0] == "fmt")
	case "generate":
		return generate(root)
	case "check-generated":
		return checkGenerated(root)
	case "notices":
		return notices(true)
	case "license":
		return license()
	case "clean":
		for _, name := range []string{"bin", ".tools"} {
			info, err := os.Lstat(name)
			if errors.Is(err, os.ErrNotExist) {
				continue
			}
			if err != nil {
				return err
			}
			if !info.IsDir() || info.Mode()&os.ModeSymlink != 0 {
				return fmt.Errorf("refusing to clean non-directory %s", name)
			}
			if err := os.RemoveAll(filepath.Join(root, name)); err != nil {
				return err
			}
		}
		return nil
	default:
		return errors.New("unknown developer command")
	}
}

func repositoryRoot() (string, error) {
	directory, err := os.Getwd()
	if err != nil {
		return "", err
	}
	for {
		data, err := os.ReadFile(filepath.Join(directory, "go.mod"))
		firstLine, _, _ := strings.Cut(string(data), "\n")
		if err == nil && strings.TrimSpace(firstLine) == "module github.com/blackoutsecure/int-buoybsd" {
			return directory, nil
		}
		if err != nil && !errors.Is(err, os.ErrNotExist) {
			return "", err
		}
		parent := filepath.Dir(directory)
		if parent == directory {
			return "", errors.New("run this helper inside the BuoyBSD repository")
		}
		directory = parent
	}
}

func command(name string, args []string, env []string, capture bool) ([]byte, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Minute)
	defer cancel()
	fmt.Fprintln(os.Stderr, ">", name, strings.Join(args, " "))
	cmd := exec.CommandContext(ctx, name, args...)
	cmd.Env = env
	cmd.Stderr = os.Stderr
	if capture {
		result, err := cmd.Output()
		if ctx.Err() != nil {
			return nil, fmt.Errorf("%s exceeded the 600-second deadline: %w", name, ctx.Err())
		}
		return result, err
	}
	cmd.Stdout = os.Stdout
	err := cmd.Run()
	if ctx.Err() != nil {
		return nil, fmt.Errorf("%s exceeded the 600-second deadline: %w", name, ctx.Err())
	}
	return nil, err
}

func build(goos, arch string) error {
	env := os.Environ()
	directory := "bin"
	extension := ""
	if goos == "" {
		if runtime.GOOS == "windows" {
			extension = ".exe"
		}
	} else {
		directory = filepath.Join(directory, goos+"_"+arch)
		env = setEnv(env, "GOOS", goos)
		env = setEnv(env, "GOARCH", arch)
		env = setEnv(env, "CGO_ENABLED", "0")
	}
	if err := os.MkdirAll(directory, 0755); err != nil {
		return err
	}
	for _, name := range []string{"buoybsd", "buoyctl"} {
		_, err := command("go", []string{"build", "-trimpath", "-o", filepath.Join(directory, name+extension), "./cmd/" + name}, env, false)
		if err != nil {
			return err
		}
	}
	return nil
}

func setEnv(env []string, name, value string) []string {
	filtered := make([]string, 0, len(env)+1)
	for _, item := range env {
		key, _, _ := strings.Cut(item, "=")
		if !strings.EqualFold(key, name) {
			filtered = append(filtered, item)
		}
	}
	return append(filtered, name+"="+value)
}

func sourceFiles() ([]string, error) {
	var files []string
	err := filepath.WalkDir(".", func(path string, entry fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if entry.IsDir() {
			switch entry.Name() {
			case ".git", ".tools", "bin":
				return filepath.SkipDir
			}
			return nil
		}
		if strings.HasSuffix(path, ".go") || strings.HasSuffix(path, ".proto") {
			files = append(files, path)
		}
		return nil
	})
	return files, err
}

func format(write bool) error {
	files, err := sourceFiles()
	if err != nil {
		return err
	}
	flag := "-l"
	if write {
		flag = "-w"
	}
	var goFiles []string
	for _, file := range files {
		if strings.HasSuffix(file, ".go") {
			goFiles = append(goFiles, file)
		}
	}
	result, err := command("gofmt", append([]string{flag}, goFiles...), nil, true)
	if err != nil {
		return err
	}
	if len(result) != 0 {
		fmt.Fprint(os.Stderr, string(result))
		return errors.New("go formatting differs; run make fmt")
	}
	return nil
}

func generate(output string) error {
	protoc, err := exec.LookPath("protoc")
	if err != nil {
		return errors.New("protoc 36.2 is required; install the checksum-verified official archive and add its bin directory to PATH")
	}
	version, err := command(protoc, []string{"--version"}, nil, true)
	if err != nil {
		return err
	}
	if strings.TrimSpace(string(version)) != protocVersion {
		return errors.New("protobuf generation requires exactly protoc 36.2")
	}
	if err := os.MkdirAll(".tools", 0755); err != nil {
		return err
	}
	plugin, err := filepath.Abs(filepath.Join(".tools", "protoc-gen-go"))
	if err != nil {
		return err
	}
	if runtime.GOOS == "windows" {
		plugin += ".exe"
	}
	if _, err := command("go", []string{"build", "-o", plugin, "google.golang.org/protobuf/cmd/protoc-gen-go"}, nil, false); err != nil {
		return err
	}
	version, err = command(plugin, []string{"--version"}, nil, true)
	if err != nil {
		return err
	}
	parts := strings.Fields(string(version))
	if len(parts) != 2 || strings.TrimSuffix(parts[0], ".exe")+" "+parts[1] != generatorVersion {
		return errors.New("protobuf generation requires protoc-gen-go v1.36.12")
	}
	include := filepath.Clean(filepath.Join(filepath.Dir(protoc), "..", "include"))
	if info, err := os.Stat(filepath.Join(include, "google", "protobuf", "timestamp.proto")); err != nil || !info.Mode().IsRegular() {
		return errors.New("protoc installation must include google/protobuf/timestamp.proto under its sibling include directory")
	}
	args := []string{
		"--proto_path=.", "--proto_path=" + include,
		"--plugin=protoc-gen-go=" + plugin,
		"--go_out=" + output, "--go_opt=paths=source_relative",
		"api/v1beta1/event.proto",
	}
	_, err = command(protoc, args, nil, false)
	return err
}

func checkGenerated(root string) (result error) {
	directory, err := os.MkdirTemp("", "buoybsd-protobuf-")
	if err != nil {
		return err
	}
	defer func() { result = errors.Join(result, os.RemoveAll(directory)) }()
	if err := generate(directory); err != nil {
		return err
	}
	path := filepath.Join("api", "v1beta1", "event.pb.go")
	expected, err := os.ReadFile(filepath.Join(directory, path))
	if err != nil {
		return err
	}
	actual, err := os.ReadFile(filepath.Join(root, path))
	if err != nil {
		return err
	}
	if !bytes.Equal(actual, expected) {
		return errors.New("generated protobuf differs; run make generate and commit the result")
	}
	fmt.Println("Generated protobuf is reproducible.")
	return nil
}

func notices(write bool) error {
	for _, entry := range []struct {
		module string
		target string
	}{
		{"google.golang.org/protobuf", "protobuf.txt"},
		{"go.yaml.in/yaml/v3", "yaml.txt"},
		{"", "go.txt"},
	} {
		args := []string{"env", "GOROOT"}
		if entry.module != "" {
			args = []string{"list", "-m", "-f", "{{.Dir}}", entry.module}
		}
		directory, err := command("go", args, nil, true)
		if err != nil {
			return err
		}
		data, err := os.ReadFile(filepath.Join(strings.TrimSpace(string(directory)), "LICENSE"))
		if err != nil {
			return err
		}
		target := filepath.Join("licenses", entry.target)
		if write {
			if err := os.WriteFile(target, data, 0644); err != nil {
				return err
			}
			continue
		}
		actual, err := os.ReadFile(target)
		if err != nil {
			return fmt.Errorf("license snapshot missing; run go run ./tools/dev notices: %w", err)
		}
		if !bytes.Equal(actual, data) {
			return fmt.Errorf("%s differs from the pinned dependency; refresh and review notices", target)
		}
	}
	return nil
}

func license() error {
	data, err := os.ReadFile("LICENSE")
	if err != nil {
		return err
	}
	if !bytes.Contains(data, []byte("Apache License")) || !bytes.Contains(data, []byte("Version 2.0")) {
		return errors.New("LICENSE must contain Apache License 2.0")
	}
	data, err = os.ReadFile("NOTICE")
	if err != nil {
		return err
	}
	if !bytes.Contains(data, []byte("BlackoutSecure contributors")) {
		return errors.New("NOTICE must retain original project attribution")
	}
	files, err := sourceFiles()
	if err != nil {
		return err
	}
	for _, file := range files {
		data, err := os.ReadFile(file)
		if err != nil {
			return err
		}
		if strings.HasSuffix(file, ".pb.go") && bytes.Contains(data, []byte("Code generated by protoc-gen-go")) {
			continue
		}
		firstLine, _, _ := strings.Cut(string(data), "\n")
		if strings.TrimSpace(firstLine) != "// SPDX-License-Identifier: Apache-2.0" {
			return fmt.Errorf("%s lacks the original-code SPDX header", file)
		}
	}
	if err := notices(false); err != nil {
		return err
	}
	fmt.Println("Original-code headers and runtime dependency notices match.")
	return nil
}
