// SPDX-License-Identifier: Apache-2.0

package config

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"net"
	"net/url"
	"os"
	"path"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/blackoutsecure/int-buoybsd/internal/collector"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
	"go.yaml.in/yaml/v3"
)

const MaxConfigBytes = 1 << 20

type Config struct {
	Version   int           `yaml:"version"`
	Collector Collector     `yaml:"collector"`
	Pipeline  Pipeline      `yaml:"pipeline"`
	Policies  []Policy      `yaml:"policies"`
	Detectors []string      `yaml:"detectors"`
	Outputs   []Destination `yaml:"outputs"`
}

type Collector struct {
	Type           string `yaml:"type"`
	Path           string `yaml:"path"`
	MaxRecordBytes int    `yaml:"max_record_bytes"`
}

type Pipeline struct {
	QueueSize int `yaml:"queue_size"`
}

type Policy struct {
	Name         string   `yaml:"name"`
	EventNames   []string `yaml:"event_names"`
	Collectors   []string `yaml:"collectors"`
	Executables  []string `yaml:"executables"`
	UIDs         []uint32 `yaml:"uids"`
	GIDs         []uint32 `yaml:"gids"`
	JailIDs      []uint32 `yaml:"jail_ids"`
	ProcessNames []string `yaml:"process_names"`
}

type Duration time.Duration

func (d Duration) MarshalYAML() (any, error) {
	return time.Duration(d).String(), nil
}

func (d *Duration) UnmarshalYAML(value *yaml.Node) error {
	if value.Tag != "!!str" {
		return errors.New("duration must be a string with a unit")
	}
	parsed, err := time.ParseDuration(value.Value)
	if err != nil {
		return errors.New("invalid duration")
	}
	*d = Duration(parsed)
	return nil
}

type Destination struct {
	Name      string            `yaml:"name"`
	Type      string            `yaml:"type"`
	Format    string            `yaml:"format"`
	Path      string            `yaml:"path"`
	URL       string            `yaml:"url"`
	Timeout   Duration          `yaml:"timeout"`
	Retries   int               `yaml:"retries"`
	Headers   map[string]string `yaml:"headers"`
	AllowHTTP bool              `yaml:"allow_http"`
}

func Default() Config {
	return Config{
		Version:   1,
		Collector: Collector{Type: "replay", MaxRecordBytes: 65536},
		Pipeline:  Pipeline{QueueSize: 64},
		Detectors: []string{"shell-exec"},
		Outputs:   []Destination{{Name: "stdout", Type: "stdout", Format: "json"}},
	}
}

func Load(reader io.Reader) (Config, error) {
	data, err := io.ReadAll(io.LimitReader(reader, MaxConfigBytes+1))
	if err != nil {
		return Config{}, fmt.Errorf("read configuration: %w", err)
	}
	if len(data) > MaxConfigBytes {
		return Config{}, errors.New("configuration exceeds 1048576 bytes")
	}
	result := Default()
	decoder := yaml.NewDecoder(bytes.NewReader(data))
	decoder.KnownFields(true)
	if err := decoder.Decode(&result); err != nil {
		// YAML errors can quote scalar values containing credentials.
		return Config{}, errors.New("invalid YAML configuration: check syntax, field names, and types")
	}
	if err := decoder.Decode(new(any)); !errors.Is(err, io.EOF) {
		return Config{}, errors.New("configuration must contain exactly one YAML document")
	}
	return result, nil
}

func (c Config) Validate() error {
	if c.Version != 1 {
		return errors.New("configuration version must be 1")
	}
	if c.Collector.Type != "replay" {
		return errors.New("only the replay collector is implemented; no live collector is available")
	}
	if strings.TrimSpace(c.Collector.Path) == "" || len(c.Collector.Path) > 4096 {
		return errors.New("collector.path is required and must be at most 4096 bytes")
	}
	if c.Collector.MaxRecordBytes < collector.MinRecordBytes || c.Collector.MaxRecordBytes > collector.MaxRecordBytes {
		return errors.New("collector.max_record_bytes must be between 256 and 1048576")
	}
	if c.Pipeline.QueueSize < 1 || c.Pipeline.QueueSize > 4096 {
		return errors.New("pipeline.queue_size must be between 1 and 4096")
	}
	if int64(c.Pipeline.QueueSize)*int64(c.Collector.MaxRecordBytes) > 64<<20 {
		return errors.New("queue_size times max_record_bytes must not exceed 64 MiB")
	}
	if err := ValidatePolicies(c.Policies); err != nil {
		return err
	}
	if len(c.Detectors) > 1 || (len(c.Detectors) == 1 && c.Detectors[0] != "shell-exec") {
		return errors.New("only the shell-exec detector is implemented; use [] to disable it")
	}
	if len(c.Outputs) < 1 || len(c.Outputs) > 8 {
		return errors.New("between 1 and 8 outputs are required")
	}
	names := make(map[string]bool)
	stdout := 0
	for i, output := range c.Outputs {
		if err := output.Validate(); err != nil {
			return fmt.Errorf("output %d: %w", i+1, err)
		}
		if names[output.Name] {
			return errors.New("output names must be unique")
		}
		names[output.Name] = true
		if output.Type == "stdout" {
			stdout++
		}
	}
	if stdout > 1 {
		return errors.New("only one stdout output is allowed; do not mix JSON and table streams")
	}
	return nil
}

func ValidatePolicies(policies []Policy) error {
	if len(policies) > 64 {
		return errors.New("at most 64 policies are allowed")
	}
	names := make(map[string]bool)
	for i, policy := range policies {
		if err := policy.Validate(); err != nil {
			return fmt.Errorf("policy %d: %w", i+1, err)
		}
		if names[policy.Name] {
			return errors.New("policy names must be unique")
		}
		names[policy.Name] = true
	}
	return nil
}

func (p Policy) Validate() error {
	if p.Name == "" || len(p.Name) > 64 {
		return errors.New("name must contain 1-64 bytes")
	}
	for _, values := range [][]string{p.EventNames, p.Collectors, p.Executables, p.ProcessNames} {
		if len(values) > 128 {
			return errors.New("at most 128 values per selector are allowed")
		}
		for _, value := range values {
			if value == "" || len(value) > collector.MaxFieldBytes {
				return errors.New("selector values must contain 1-4096 bytes")
			}
		}
	}
	for _, values := range [][]uint32{p.UIDs, p.GIDs, p.JailIDs} {
		if len(values) > 128 {
			return errors.New("at most 128 numeric values per selector are allowed")
		}
	}
	for _, name := range p.EventNames {
		if !event.Known(name) {
			return errors.New("unsupported event selector")
		}
	}
	for _, name := range p.Collectors {
		if name != "replay" {
			return errors.New("unsupported collector selector")
		}
	}
	for _, patterns := range [][]string{p.Executables, p.ProcessNames} {
		for _, pattern := range patterns {
			if _, err := path.Match(pattern, ""); err != nil {
				return errors.New("invalid glob selector")
			}
		}
	}
	return nil
}

func (d Destination) Validate() error {
	if d.Name == "" || len(d.Name) > 64 {
		return errors.New("name must contain 1-64 bytes")
	}
	switch d.Type {
	case "stdout", "file":
		if d.Format != "json" && !(d.Type == "stdout" && d.Format == "table") {
			return errors.New("stdout supports json/table; files support json only")
		}
		if d.Type == "file" && (d.Path == "" || len(d.Path) > 4096) {
			return errors.New("file output requires a path of 1-4096 bytes")
		}
		if d.Type == "stdout" && d.Path != "" {
			return errors.New("stdout output cannot set path")
		}
		if d.URL != "" || d.Timeout != 0 || d.Retries != 0 || len(d.Headers) != 0 || d.AllowHTTP {
			return errors.New("non-webhook output cannot set webhook options")
		}
	case "webhook":
		if d.Format != "json" || d.Path != "" {
			return errors.New("webhook requires format json and cannot set path")
		}
		if len(d.URL) > 4096 {
			return errors.New("webhook URL exceeds 4096 bytes")
		}
		parsed, err := url.Parse(d.URL)
		if err != nil || parsed.Hostname() == "" || parsed.User != nil || parsed.Fragment != "" {
			return errors.New("webhook requires an absolute URL without userinfo or fragment")
		}
		if port := parsed.Port(); port != "" {
			value, err := strconv.Atoi(port)
			if err != nil || value < 1 || value > 65535 {
				return errors.New("webhook port must be between 1 and 65535")
			}
		}
		loopback := strings.EqualFold(parsed.Hostname(), "localhost")
		if ip := net.ParseIP(parsed.Hostname()); ip != nil {
			loopback = ip.IsLoopback()
		}
		if parsed.Scheme != "https" && !(parsed.Scheme == "http" && d.AllowHTTP && loopback) {
			return errors.New("webhook requires HTTPS; explicitly enabled HTTP is limited to loopback testing")
		}
		if d.Timeout < Duration(time.Millisecond) || d.Timeout > Duration(30*time.Second) {
			return errors.New("webhook timeout must be between 1ms and 30s")
		}
		if d.Retries < 0 || d.Retries > 5 {
			return errors.New("webhook retries must be between 0 and 5")
		}
		if len(d.Headers) > 32 {
			return errors.New("at most 32 webhook headers are allowed")
		}
		seen := make(map[string]bool)
		for name, value := range d.Headers {
			if !validHeader(name, value) {
				return errors.New("invalid webhook header name or value")
			}
			canonical := strings.ToLower(name)
			if seen[canonical] {
				return errors.New("duplicate case-insensitive webhook header")
			}
			seen[canonical] = true
			switch canonical {
			case "host", "content-type", "content-length", "transfer-encoding", "connection":
				return errors.New("webhook headers cannot override transport or JSON content headers")
			}
		}
	default:
		return errors.New("unsupported destination type")
	}
	return nil
}

func validHeader(name, value string) bool {
	if name == "" || len(name) > 128 || len(value) > 4096 {
		return false
	}
	for _, char := range name {
		if char > 127 || !(char >= 'a' && char <= 'z' || char >= 'A' && char <= 'Z' ||
			char >= '0' && char <= '9' || strings.ContainsRune("!#$%&'*+-.^_`|~", char)) {
			return false
		}
	}
	for _, char := range value {
		if char < 32 || char == 127 {
			return false
		}
	}
	return true
}

func (c Config) ValidatePaths(configPath string) error {
	inputs := []string{c.Collector.Path}
	if configPath != "" {
		inputs = append(inputs, configPath)
	}
	for _, input := range inputs {
		info, err := os.Stat(input)
		if err != nil {
			return fmt.Errorf("stat input file: %w", err)
		}
		if !info.Mode().IsRegular() {
			return errors.New("input must be a regular file")
		}
		for _, output := range c.Outputs {
			if output.Type != "file" {
				continue
			}
			outInfo, err := os.Stat(output.Path)
			if err != nil && !errors.Is(err, os.ErrNotExist) {
				return fmt.Errorf("stat output file: %w", err)
			}
			inPath, err := filepath.Abs(input)
			if err != nil {
				return err
			}
			outPath, err := filepath.Abs(output.Path)
			if err != nil {
				return err
			}
			if inPath == outPath || outInfo != nil && os.SameFile(info, outInfo) {
				return errors.New("file output cannot also be a configuration or replay input")
			}
			if outInfo != nil && !outInfo.Mode().IsRegular() {
				return errors.New("file output must be a regular file")
			}
		}
	}
	return nil
}
