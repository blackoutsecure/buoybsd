// SPDX-License-Identifier: Apache-2.0

package config

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"go.yaml.in/yaml/v3"
)

func validConfig() Config {
	cfg := Default()
	cfg.Collector.Path = "synthetic.ndjson"
	return cfg
}

func webhook() Destination {
	return Destination{Name: "hook", Type: "webhook", Format: "json", URL: "https://example.invalid/events",
		Timeout: Duration(time.Second), Retries: 2}
}

func TestConfigStrictLoadAndRoundTrip(t *testing.T) {
	cfg := validConfig()
	cfg.Outputs = append(cfg.Outputs, webhook())
	data, err := yaml.Marshal(cfg)
	if err != nil {
		t.Fatal(err)
	}
	loaded, err := Load(bytes.NewReader(data))
	if err != nil {
		t.Fatal(err)
	}
	if err := loaded.Validate(); err != nil {
		t.Fatal(err)
	}
	if loaded.Outputs[1].Timeout != Duration(time.Second) {
		t.Fatal("duration did not round trip")
	}
	for _, input := range []string{
		"unknown: true\n",
		"collector:\n  unknown: true\n",
		"version: 1\nversion: 2\n",
		"version: 1\n---\nversion: 1\n",
		"outputs:\n  - headers:\n      Authorization: [fixture-secret]\n",
		"outputs:\n  - timeout: fixture-secret\n",
		"outputs: [fixture-secret\n",
		"",
	} {
		if _, err := Load(strings.NewReader(input)); err == nil || strings.Contains(err.Error(), "fixture-secret") {
			t.Fatalf("invalid/secret-bearing configuration was accepted or echoed: %v", err)
		}
	}
	if _, err := Load(strings.NewReader(strings.Repeat(" ", MaxConfigBytes+1))); err == nil {
		t.Fatal("oversized configuration accepted")
	}
}

func TestConfigurationBoundsAndContradictions(t *testing.T) {
	tests := map[string]func(*Config){
		"version":        func(c *Config) { c.Version = 2 },
		"native stub":    func(c *Config) { c.Collector.Type = "audit" },
		"no path":        func(c *Config) { c.Collector.Path = "" },
		"small record":   func(c *Config) { c.Collector.MaxRecordBytes = 255 },
		"large record":   func(c *Config) { c.Collector.MaxRecordBytes = 1048577 },
		"zero queue":     func(c *Config) { c.Pipeline.QueueSize = 0 },
		"large queue":    func(c *Config) { c.Pipeline.QueueSize = 4097 },
		"memory budget":  func(c *Config) { c.Pipeline.QueueSize = 65; c.Collector.MaxRecordBytes = 1 << 20 },
		"unknown engine": func(c *Config) { c.Detectors = []string{"rego"} },
		"duplicate detector": func(c *Config) {
			c.Detectors = []string{"shell-exec", "shell-exec"}
		},
		"duplicate policies": func(c *Config) { c.Policies = []Policy{{Name: "same"}, {Name: "same"}} },
		"unknown event":      func(c *Config) { c.Policies = []Policy{{Name: "unknown", EventNames: []string{"process_exit"}}} },
		"invalid glob":       func(c *Config) { c.Policies = []Policy{{Name: "bad", Executables: []string{"["}}} },
		"no outputs":         func(c *Config) { c.Outputs = nil },
		"unknown output":     func(c *Config) { c.Outputs[0].Type = "forward" },
		"duplicate outputs":  func(c *Config) { c.Outputs = append(c.Outputs, c.Outputs[0]) },
		"mixed stdout": func(c *Config) {
			c.Outputs = append(c.Outputs, Destination{Name: "other", Type: "stdout", Format: "table"})
		},
	}
	for name, modify := range tests {
		t.Run(name, func(t *testing.T) {
			cfg := validConfig()
			modify(&cfg)
			if err := cfg.Validate(); err == nil {
				t.Fatal("invalid configuration accepted")
			}
		})
	}
	cfg := validConfig()
	cfg.Collector.MaxRecordBytes, cfg.Pipeline.QueueSize = 1<<20, 64
	if err := cfg.Validate(); err != nil {
		t.Fatalf("exactly 64 MiB raw queue budget must be accepted: %v", err)
	}
	cfg.Collector.MaxRecordBytes, cfg.Pipeline.QueueSize = 256, 4096
	if err := cfg.Validate(); err != nil {
		t.Fatal(err)
	}
}

func TestDestinationValidation(t *testing.T) {
	for name, modify := range map[string]func(*Destination){
		"userinfo":       func(d *Destination) { d.URL = "https://user:fixture-secret@example.invalid" },
		"fragment":       func(d *Destination) { d.URL += "#fixture-secret" },
		"relative":       func(d *Destination) { d.URL = "/events" },
		"malformed":      func(d *Destination) { d.URL = "https://%" },
		"bad port":       func(d *Destination) { d.URL = "https://example.invalid:65536/events" },
		"unencrypted":    func(d *Destination) { d.URL = "http://127.0.0.1/events" },
		"remote HTTP":    func(d *Destination) { d.URL = "http://example.invalid/events"; d.AllowHTTP = true },
		"no timeout":     func(d *Destination) { d.Timeout = 0 },
		"long timeout":   func(d *Destination) { d.Timeout = Duration(31 * time.Second) },
		"short timeout":  func(d *Destination) { d.Timeout = Duration(time.Microsecond) },
		"negative retry": func(d *Destination) { d.Retries = -1 },
		"many retries":   func(d *Destination) { d.Retries = 6 },
		"invalid header": func(d *Destination) { d.Headers = map[string]string{"Authorization": "fixture-secret\r\nx: y"} },
		"header name":    func(d *Destination) { d.Headers = map[string]string{"bad name": "fixture-secret"} },
		"reserved":       func(d *Destination) { d.Headers = map[string]string{"Content-Type": "fixture-secret"} },
		"case duplicate": func(d *Destination) {
			d.Headers = map[string]string{"Authorization": "fixture-secret", "authorization": "other"}
		},
		"wrong format": func(d *Destination) { d.Format = "table" },
		"extra path":   func(d *Destination) { d.Path = "events.ndjson" },
	} {
		t.Run(name, func(t *testing.T) {
			spec := webhook()
			modify(&spec)
			if err := spec.Validate(); err == nil || strings.Contains(err.Error(), "fixture-secret") {
				t.Fatalf("invalid destination accepted or credential echoed: %v", err)
			}
		})
	}
	for _, address := range []string{"http://localhost:8080/events", "http://127.0.0.1:8080/events", "http://[::1]:8080/events"} {
		spec := webhook()
		spec.URL, spec.AllowHTTP = address, true
		if err := spec.Validate(); err != nil {
			t.Fatalf("explicit loopback HTTP was rejected: %v", err)
		}
	}
	spec := webhook()
	spec.Retries, spec.Timeout = 5, Duration(time.Millisecond)
	if err := spec.Validate(); err != nil {
		t.Fatal(err)
	}
	for _, spec := range []Destination{
		{Name: "file", Type: "file", Format: "table", Path: "events"},
		{Name: "stdout", Type: "stdout", Format: "json", Path: "events"},
		{Name: "file", Type: "file", Format: "json", Path: "events", URL: "https://example.invalid"},
	} {
		if err := spec.Validate(); err == nil {
			t.Fatal("contradictory output options accepted")
		}
	}
}

func TestPathValidationPreventsInputOutputAlias(t *testing.T) {
	directory := t.TempDir()
	input := filepath.Join(directory, "input.ndjson")
	if err := os.WriteFile(input, []byte("synthetic"), 0600); err != nil {
		t.Fatal(err)
	}
	cfg := validConfig()
	cfg.Collector.Path = input
	cfg.Outputs = []Destination{{Name: "file", Type: "file", Format: "json", Path: input}}
	if err := cfg.ValidatePaths(""); err == nil {
		t.Fatal("input/output path alias accepted")
	}
	cfg.Outputs[0].Path = filepath.Join(directory, "new-output.ndjson")
	if err := cfg.ValidatePaths(""); err != nil {
		t.Fatal(err)
	}
	alias := filepath.Join(directory, "alias.ndjson")
	if err := os.Link(input, alias); err != nil {
		t.Fatal(err)
	}
	cfg.Outputs[0].Path = alias
	if err := cfg.ValidatePaths(""); err == nil {
		t.Fatal("hard-linked input/output alias accepted")
	}
	cfg.Outputs[0].Path = directory
	if err := cfg.ValidatePaths(""); err == nil {
		t.Fatal("directory output accepted")
	}
	cfg.Outputs[0].Path = filepath.Join(directory, "new-output.ndjson")
	cfg.Collector.Path = directory
	if err := cfg.ValidatePaths(""); err == nil {
		t.Fatal("directory input accepted")
	}
}
