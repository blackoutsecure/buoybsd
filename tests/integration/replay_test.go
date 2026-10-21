// SPDX-License-Identifier: Apache-2.0

//go:build integration

package integration

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"sync/atomic"
	"testing"
	"time"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"github.com/blackoutsecure/int-buoybsd/internal/cli"
	"github.com/blackoutsecure/int-buoybsd/internal/pipeline"
	"github.com/blackoutsecure/int-buoybsd/pkg/config"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
	"go.yaml.in/yaml/v3"
	"google.golang.org/protobuf/encoding/protojson"
	"google.golang.org/protobuf/proto"
)

func TestReplayToJSONFileAndWebhook(t *testing.T) {
	input, err := filepath.Abs(filepath.Join("..", "fixtures", "process_exec.synthetic.ndjson"))
	if err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(input)
	if err != nil {
		t.Fatal(err)
	}
	expectedRecords := bytes.SplitAfter(raw, []byte("\n"))
	received := make(chan []byte, 2)
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		data, err := io.ReadAll(io.LimitReader(request.Body, 1<<20))
		if err != nil {
			writer.WriteHeader(http.StatusBadRequest)
			return
		}
		received <- data
		writer.WriteHeader(http.StatusNoContent)
	}))
	defer server.Close()
	cfg := config.Default()
	cfg.Collector.Path = input
	cfg.Pipeline.QueueSize = 1
	cfg.Policies = []config.Policy{{Name: "exec", EventNames: []string{event.ProcessExec}}}
	filePath := filepath.Join(t.TempDir(), "events.ndjson")
	cfg.Outputs = append(cfg.Outputs,
		config.Destination{Name: "file", Type: "file", Format: "json", Path: filePath},
		config.Destination{Name: "hook", Type: "webhook", Format: "json", URL: server.URL,
			Timeout: config.Duration(time.Second), AllowHTTP: true},
	)
	configPath := writeConfig(t, cfg)
	var stdout, stderr bytes.Buffer
	if err := cli.RunSensor(context.Background(), []string{"--config", configPath}, &stdout, &stderr); err != nil {
		t.Fatal(err)
	}
	lines := bytes.Split(bytes.TrimSuffix(stdout.Bytes(), []byte("\n")), []byte("\n"))
	if len(lines) != 2 {
		t.Fatal("replay did not emit exactly two JSON objects")
	}
	file, err := os.ReadFile(filePath)
	if err != nil {
		t.Fatal(err)
	}
	fileLines := bytes.Split(bytes.TrimSuffix(file, []byte("\n")), []byte("\n"))
	if len(fileLines) != len(lines) {
		t.Fatal("stdout and file destinations received different event counts")
	}
	for i, line := range lines {
		var normalized v1.Event
		if err := protojson.Unmarshal(line, &normalized); err != nil {
			t.Fatal(err)
		}
		if normalized.SchemaVersion != event.SchemaVersion || normalized.Name != event.ProcessExec ||
			normalized.Source.Evidence != "synthetic" || normalized.Source.ReplaySequence != uint64(i+1) ||
			!bytes.Equal(normalized.RawRecord.Data, expectedRecords[i]) ||
			len(normalized.MatchedPolicies) != 1 || normalized.MatchedPolicies[0] != "exec" {
			t.Fatal("vertical slice lost schema, source, raw bytes, ordering, or policy metadata")
		}
		if i == 0 && (normalized.Process.Uid == nil || normalized.Process.GetUid() != 0 ||
			len(normalized.Detections) != 1 || normalized.Detections[0].DetectedFrom.RecordId != normalized.RecordId) {
			t.Fatal("observed root identity or detector source reference was lost")
		}
		assertJSONEvent(t, fileLines[i], &normalized)
		select {
		case body := <-received:
			assertJSONEvent(t, body, &normalized)
		case <-time.After(time.Second):
			t.Fatal("webhook did not receive an event")
		}
	}
	stats := diagnostics(t, stderr.Bytes())
	if stats != (pipeline.Stats{EventsReceived: 2, EventsNormalized: 2, EventsDelivered: 2,
		DetectionsGenerated: 1, DeliveryAttempts: 6}) {
		t.Fatalf("wrong end-to-end metrics: %+v", stats)
	}
}

func assertJSONEvent(t *testing.T, data []byte, expected *v1.Event) {
	t.Helper()
	var decoded v1.Event
	if err := protojson.Unmarshal(data, &decoded); err != nil {
		t.Fatal(err)
	}
	if !proto.Equal(&decoded, expected) {
		t.Fatal("output destinations received semantically different events")
	}
}

func TestReplayPolicyTableAndDeliveryFailure(t *testing.T) {
	input, err := filepath.Abs(filepath.Join("..", "fixtures", "process_exec.synthetic.ndjson"))
	if err != nil {
		t.Fatal(err)
	}
	cfg := config.Default()
	cfg.Collector.Path = input
	cfg.Policies = []config.Policy{{Name: "user", UIDs: []uint32{1001}}}
	cfg.Outputs[0].Format = "table"
	var stdout, stderr bytes.Buffer
	if err := cli.RunSensor(context.Background(), []string{"--config", writeConfig(t, cfg)}, &stdout, &stderr); err != nil {
		t.Fatal(err)
	}
	if bytes.Count(stdout.Bytes(), []byte("\n")) != 2 || !bytes.Contains(stdout.Bytes(), []byte("/usr/bin/true")) ||
		bytes.Contains(stdout.Bytes(), []byte("/bin/sh")) {
		t.Fatal("table/policy vertical slice emitted incorrect rows")
	}
	stats := diagnostics(t, stderr.Bytes())
	if stats.EventsFiltered != 1 || stats.EventsDelivered != 1 {
		t.Fatal(stats)
	}

	var calls atomic.Uint32
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		calls.Add(1)
		writer.WriteHeader(http.StatusBadRequest)
	}))
	defer server.Close()
	cfg.Policies = nil
	cfg.Outputs = []config.Destination{{Name: "hook", Type: "webhook", Format: "json",
		URL: server.URL, Timeout: config.Duration(time.Second), Retries: 5, AllowHTTP: true}}
	stdout.Reset()
	stderr.Reset()
	if err := cli.RunSensor(context.Background(), []string{"--config", writeConfig(t, cfg)}, &stdout, &stderr); err == nil {
		t.Fatal("delivery failure was reported as success")
	}
	stats = diagnostics(t, stderr.Bytes())
	if calls.Load() != 1 || stats.EventsDelivered != 0 || stats.DestinationErrors != 1 || stats.EventsDropped == 0 {
		t.Fatalf("permanent failure was retried or hidden: %+v", stats)
	}
}

func writeConfig(t *testing.T, cfg config.Config) string {
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

func diagnostics(t *testing.T, data []byte) pipeline.Stats {
	t.Helper()
	var entry struct {
		Metrics pipeline.Stats `json:"metrics"`
	}
	if err := json.Unmarshal(data, &entry); err != nil {
		t.Fatal(err)
	}
	return entry.Metrics
}
