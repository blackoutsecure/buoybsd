// SPDX-License-Identifier: Apache-2.0

package normalize

import (
	"bytes"
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"testing"
	"time"

	"github.com/blackoutsecure/int-buoybsd/internal/collector"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
	"google.golang.org/protobuf/encoding/protojson"
)

func ptr[T any](value T) *T { return &value }

func observation() collector.RawEvent {
	return collector.RawEvent{
		Name: event.ProcessExec, Timestamp: ptr(time.Unix(1700000000, 123456789)),
		Collector: "replay", OperatingSystem: "freebsd", Evidence: "synthetic", Sequence: 1,
		SourceID: ptr("synthetic"), Host: &collector.Host{Name: ptr("fixture-host")},
		Process: &collector.Process{PID: ptr(uint32(42)), UID: ptr(uint32(0)), ExecutablePath: ptr("/bin/sh")},
		Jail:    &collector.Jail{ID: ptr(uint32(0))},
		Exec:    &collector.ProcessExec{Argv: []string{"sh"}, ArgvComplete: ptr(true), Success: ptr(true)},
		Record:  []byte("original synthetic record\r\n"), MediaType: "application/x-buoybsd-replay+json",
	}
}

func TestNormalizePresenceSourceAndRawPreservation(t *testing.T) {
	raw := observation()
	normalized, err := Event(raw)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(normalized.RawRecord.Data, raw.Record) ||
		normalized.RecordId != fmt.Sprintf("sha256:%x", sha256.Sum256(raw.Record)) {
		t.Fatal("raw bytes or content reference changed")
	}
	if normalized.Timestamp.Nanos != 123456789 || normalized.Source.Evidence != "synthetic" ||
		normalized.SchemaVersion != event.SchemaVersion || normalized.Jail.Id == nil || *normalized.Jail.Id != 0 {
		t.Fatal("source context or timestamp was lost")
	}
	data, err := (protojson.MarshalOptions{UseProtoNames: true}).Marshal(normalized)
	if err != nil {
		t.Fatal(err)
	}
	var document map[string]json.RawMessage
	if err := json.Unmarshal(data, &document); err != nil {
		t.Fatal(err)
	}
	var process map[string]json.RawMessage
	if err := json.Unmarshal(document["process"], &process); err != nil {
		t.Fatal(err)
	}
	if string(process["uid"]) != "0" || process["euid"] != nil {
		t.Fatal("absent credentials were fabricated or UID zero was omitted")
	}
	raw.Record[0] = 'X'
	raw.Exec.Argv[0] = "changed"
	*raw.Process.UID = 99
	*raw.SourceID = "changed"
	if normalized.RawRecord.Data[0] == 'X' || normalized.GetProcessExec().Argv[0] != "sh" ||
		normalized.Process.GetUid() != 0 || normalized.Source.GetSourceId() != "synthetic" {
		t.Fatal("normalized event aliases mutable raw source data")
	}
}

func TestNormalizeMissingFieldsWarnWithoutFabricating(t *testing.T) {
	raw := observation()
	raw.Process, raw.Exec, raw.Jail, raw.Host = nil, nil, nil, nil
	normalized, err := Event(raw)
	if err != nil {
		t.Fatal(err)
	}
	if normalized.Process != nil || normalized.Jail != nil || normalized.Host != nil {
		t.Fatal("missing context was fabricated")
	}
	if len(normalized.Warnings) != 4 || normalized.GetProcessExec().ArgvComplete != nil ||
		normalized.GetProcessExec().Success != nil {
		t.Fatal("missing data was not explicitly represented")
	}
}

func TestNormalizeRejectsInvalidObservation(t *testing.T) {
	tests := map[string]func(*collector.RawEvent){
		"name":      func(raw *collector.RawEvent) { raw.Name = "unknown" },
		"timestamp": func(raw *collector.RawEvent) { raw.Timestamp = nil },
		"range":     func(raw *collector.RawEvent) { raw.Timestamp = ptr(time.Date(10000, 1, 1, 0, 0, 0, 0, time.UTC)) },
		"raw":       func(raw *collector.RawEvent) { raw.Record = nil },
		"oversized": func(raw *collector.RawEvent) { raw.Record = make([]byte, collector.MaxRecordBytes+1) },
		"source":    func(raw *collector.RawEvent) { raw.Collector = "" },
		"OS":        func(raw *collector.RawEvent) { raw.OperatingSystem = "" },
		"evidence":  func(raw *collector.RawEvent) { raw.Evidence = "" },
		"format":    func(raw *collector.RawEvent) { raw.MediaType = "" },
	}
	for name, modify := range tests {
		t.Run(name, func(t *testing.T) {
			raw := observation()
			modify(&raw)
			if _, err := Event(raw); err == nil {
				t.Fatal("invalid source observation accepted")
			}
		})
	}
}

func TestNormalizeDistinguishesObservedZeroTimestampFromMissing(t *testing.T) {
	raw := observation()
	raw.Timestamp = ptr(time.Time{})
	normalized, err := Event(raw)
	if err != nil || !normalized.Timestamp.AsTime().IsZero() {
		t.Fatalf("observed year-one timestamp was treated as missing: %v", err)
	}
}
