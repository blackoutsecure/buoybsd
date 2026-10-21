// SPDX-License-Identifier: Apache-2.0

package cli

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"github.com/blackoutsecure/int-buoybsd/pkg/config"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
	"go.yaml.in/yaml/v3"
	"google.golang.org/protobuf/encoding/protojson"
)

func fixture(t *testing.T) string {
	t.Helper()
	path, err := filepath.Abs(filepath.Join("..", "..", "tests", "fixtures", "process_exec.synthetic.ndjson"))
	if err != nil {
		t.Fatal(err)
	}
	return path
}

func configFile(t *testing.T, cfg config.Config) string {
	t.Helper()
	data, err := yaml.Marshal(cfg)
	if err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(t.TempDir(), "config.yaml")
	if err := os.WriteFile(path, data, 0600); err != nil {
		t.Fatal(err)
	}
	return path
}

func TestSensorReplayJSONAndSeparateDiagnostics(t *testing.T) {
	var stdout, stderr bytes.Buffer
	if err := RunSensor(context.Background(), []string{"--replay", fixture(t)}, &stdout, &stderr); err != nil {
		t.Fatal(err)
	}
	lines := bytes.Split(bytes.TrimSuffix(stdout.Bytes(), []byte("\n")), []byte("\n"))
	if len(lines) != 2 {
		t.Fatal("expected two replay events")
	}
	for _, line := range lines {
		var event v1.Event
		if err := protojson.Unmarshal(line, &event); err != nil {
			t.Fatal(err)
		}
		if event.Source.Collector != "replay" || event.Source.Evidence != "synthetic" || event.RawRecord == nil {
			t.Fatal("source attribution or original input lost")
		}
	}
	var diagnostic struct {
		Metrics struct {
			Delivered uint64 `json:"events_delivered"`
			Dropped   uint64 `json:"events_dropped"`
		} `json:"metrics"`
	}
	if err := json.Unmarshal(stderr.Bytes(), &diagnostic); err != nil {
		t.Fatal(err)
	}
	if diagnostic.Metrics.Delivered != 2 || diagnostic.Metrics.Dropped != 0 ||
		strings.Contains(stderr.String(), "fixture-host") || strings.Contains(stderr.String(), "/bin/sh") {
		t.Fatal("diagnostics mixed with event payloads or reported wrong counters")
	}
}

func TestExplicitCLIOverridesAndFileAppend(t *testing.T) {
	cfg := config.Default()
	cfg.Collector.Path = fixture(t)
	cfg.Outputs[0].Format = "table"
	filePath := filepath.Join(t.TempDir(), "events.ndjson")
	if err := os.WriteFile(filePath, []byte("{}\n"), 0600); err != nil {
		t.Fatal(err)
	}
	cfg.Outputs = append(cfg.Outputs, config.Destination{Name: "file", Type: "file", Format: "json", Path: filePath})
	path := configFile(t, cfg)
	var stdout, stderr bytes.Buffer
	if err := RunSensor(context.Background(), []string{"--config", path}, &stdout, &stderr); err != nil {
		t.Fatal(err)
	}
	if !strings.HasPrefix(stdout.String(), "TIME\tEVENT\tPID\tEXECUTABLE\n") {
		t.Fatal("an unprovided default CLI flag overrode the configured format")
	}
	stdout.Reset()
	stderr.Reset()
	if err := RunSensor(context.Background(), []string{"--config", path, "--output", "json"}, &stdout, &stderr); err != nil {
		t.Fatal(err)
	}
	if !strings.HasPrefix(stdout.String(), "{") {
		t.Fatal("explicit format override was ignored")
	}
	file, err := os.ReadFile(filePath)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.HasPrefix(file, []byte("{}\n")) || bytes.Count(file, []byte("\n")) != 5 {
		t.Fatal("file was truncated or CLI override discarded the file destination")
	}
}

func TestSensorRejectsUnsupportedOrUnsafeInvocation(t *testing.T) {
	for name, args := range map[string][]string{
		"no fake live fallback": {},
		"bad output":            {"--replay", fixture(t), "--output", "forward"},
		"missing file":          {"--replay", filepath.Join(t.TempDir(), "missing")},
		"unknown flag":          {"--unknown"},
		"positional":            {"anything"},
		"directory config":      {"--config", t.TempDir()},
	} {
		t.Run(name, func(t *testing.T) {
			var stdout, stderr bytes.Buffer
			if err := RunSensor(context.Background(), args, &stdout, &stderr); err == nil {
				t.Fatal("unsupported invocation accepted")
			}
			if stdout.Len() != 0 {
				t.Fatal("invalid invocation produced success-shaped event output")
			}
		})
	}
	cfg := config.Default()
	cfg.Collector.Path = fixture(t)
	cfg.Outputs = []config.Destination{{Name: "alias", Type: "file", Format: "json", Path: fixture(t)}}
	before, err := os.ReadFile(fixture(t))
	if err != nil {
		t.Fatal(err)
	}
	if err := RunSensor(context.Background(), []string{"--config", configFile(t, cfg)}, io.Discard, io.Discard); err == nil {
		t.Fatal("input/output alias accepted")
	}
	after, err := os.ReadFile(fixture(t))
	if err != nil || !bytes.Equal(before, after) {
		t.Fatal("invalid setup changed the input file")
	}
}

type failedWriter struct{ err error }

func (w failedWriter) Write([]byte) (int, error) { return 0, w.err }

func TestSensorSurfacesOutputErrorAndCancellation(t *testing.T) {
	sentinel := errors.New("fixture stdout failure")
	var stderr bytes.Buffer
	if err := RunSensor(context.Background(), []string{"--replay", fixture(t)}, failedWriter{sentinel}, &stderr); !errors.Is(err, sentinel) {
		t.Fatal(err)
	}
	if !strings.Contains(stderr.String(), `"destination_errors":1`) {
		t.Fatal("destination failure was not counted")
	}
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	if err := RunSensor(ctx, []string{"--replay", fixture(t)}, io.Discard, io.Discard); !errors.Is(err, context.Canceled) {
		t.Fatal(err)
	}
}

func TestControlAndSensorVersionCommands(t *testing.T) {
	var stdout, stderr bytes.Buffer
	if err := RunSensor(context.Background(), []string{"--version"}, &stdout, &stderr); err != nil ||
		!strings.Contains(stdout.String(), event.Version) {
		t.Fatalf("sensor version requires no input: %v", err)
	}
	stdout.Reset()
	if err := RunControl([]string{"version"}, &stdout); err != nil || !strings.Contains(stdout.String(), event.Version) {
		t.Fatal(err)
	}
	stdout.Reset()
	if err := RunControl([]string{"events", "list"}, &stdout); err != nil {
		t.Fatal(err)
	}
	var definitions []event.Definition
	if err := json.Unmarshal(stdout.Bytes(), &definitions); err != nil || len(definitions) != 1 ||
		definitions[0].Name != event.ProcessExec || definitions[0].Collector != "replay" {
		t.Fatal("catalogue falsely advertised native or unimplemented events")
	}
	for _, args := range [][]string{{"status"}, {}, {"events"}, {"unknown"}} {
		if err := RunControl(args, io.Discard); err == nil {
			t.Fatal("unimplemented control operation reported success")
		}
	}
}
