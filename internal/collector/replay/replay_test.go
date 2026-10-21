// SPDX-License-Identifier: Apache-2.0

package replay

import (
	"bytes"
	"context"
	"errors"
	"io"
	"strings"
	"testing"
	"time"

	"github.com/blackoutsecure/int-buoybsd/internal/collector"
)

const validRecord = `{"version":"buoybsd.replay.v1","event":"process_exec","timestamp":"2026-01-01T00:00:00.123456789Z","source_os":"freebsd","evidence":"synthetic","process":{"pid":42,"uid":0},"exec":{"success":true}}`

func TestReplayPreservesBytesPresenceAndOrder(t *testing.T) {
	line := validRecord + "\r\n"
	source, err := New(io.NopCloser(strings.NewReader(line+validRecord)), 65536)
	if err != nil {
		t.Fatal(err)
	}
	defer func() {
		if err := source.Close(); err != nil {
			t.Error(err)
		}
	}()
	events := make(chan collector.RawEvent, 2)
	if err := source.Run(context.Background(), events); err != nil {
		t.Fatal(err)
	}
	first, second := <-events, <-events
	if string(first.Record) != line || string(second.Record) != validRecord {
		t.Fatal("raw bytes or line endings changed")
	}
	if first.Sequence != 1 || second.Sequence != 2 {
		t.Fatal("input order was not preserved")
	}
	if first.Process.UID == nil || *first.Process.UID != 0 || first.Process.EUID != nil {
		t.Fatal("present zero and missing credentials were conflated")
	}
	if first.Timestamp.Nanosecond() != 123456789 || first.Collector != "replay" || first.Evidence != "synthetic" {
		t.Fatal("timestamp or attribution changed")
	}
	if source.Stats() != (collector.Stats{Received: 2}) {
		t.Fatalf("unexpected counters: %+v", source.Stats())
	}
	if err := source.Run(context.Background(), events); err == nil {
		t.Fatal("collector reuse was accepted")
	}
}

func TestReplayRejectsInvalidRecords(t *testing.T) {
	tests := map[string]string{
		"blank":             "\n",
		"malformed":         "{",
		"scalar":            "null",
		"array":             "[]",
		"trailing value":    validRecord + " {}",
		"duplicate root":    strings.Replace(validRecord, `"event":`, `"event":"process_exec","event":`, 1),
		"duplicate nested":  strings.Replace(validRecord, `"pid":42`, `"pid":42,"pid":43`, 1),
		"noncanonical key":  strings.Replace(validRecord, `"event":`, `"Event":`, 1),
		"case duplicate":    strings.Replace(validRecord, `"event":`, `"Event":"process_exec","event":`, 1),
		"unknown root":      strings.Replace(validRecord, `"event":`, `"unrecognized":true,"event":`, 1),
		"unknown nested":    strings.Replace(validRecord, `"pid":42`, `"pid":42,"unrecognized":1`, 1),
		"bad version":       strings.Replace(validRecord, "buoybsd.replay.v1", "buoybsd.replay.v2", 1),
		"unsupported event": strings.Replace(validRecord, "process_exec", "process_exit", 1),
		"false evidence":    strings.Replace(validRecord, "synthetic", "captured", 1),
		"wrong OS":          strings.Replace(validRecord, "freebsd", "linux", 1),
		"invalid timestamp": strings.Replace(validRecord, "2026-01-01T00:00:00.123456789Z", "2026-01-01", 1),
		"no offset":         strings.Replace(validRecord, "2026-01-01T00:00:00.123456789Z", "2026-01-01T00:00:00", 1),
		"lost precision":    strings.Replace(validRecord, ".123456789Z", ".1234567891Z", 1),
		"comma fraction":    strings.Replace(validRecord, ".123456789Z", ",123456789Z", 1),
		"bad offset hour":   strings.Replace(validRecord, ".123456789Z", ".123456789+24:00", 1),
		"bad offset minute": strings.Replace(validRecord, ".123456789Z", ".123456789+00:60", 1),
		"year zero":         strings.Replace(validRecord, "2026-01-01", "0000-01-01", 1),
		"negative PID":      strings.Replace(validRecord, `"pid":42`, `"pid":-1`, 1),
		"PID overflow":      strings.Replace(validRecord, `"pid":42`, `"pid":4294967296`, 1),
		"zero PID":          strings.Replace(validRecord, `"pid":42`, `"pid":0`, 1),
		"invalid UTF8":      strings.Replace(validRecord, "freebsd", "freebsd"+string([]byte{0xff}), 1),
		"long field":        strings.Replace(validRecord, `"pid":42`, `"pid":42,"command":"`+strings.Repeat("x", 4097)+`"`, 1),
		"long argument":     strings.Replace(validRecord, `"success":true`, `"success":true,"argv":["`+strings.Repeat("x", 4097)+`"]`, 1),
		"many arguments":    strings.Replace(validRecord, `"success":true`, `"success":true,"argv":[`+strings.Repeat(`"x",`, 128)+`"x"]`, 1),
		"deep nesting":      strings.Repeat(`{"x":`, 18) + `0` + strings.Repeat("}", 18),
	}
	for name, input := range tests {
		t.Run(name, func(t *testing.T) {
			source, err := New(io.NopCloser(strings.NewReader(input)), 65536)
			if err != nil {
				t.Fatal(err)
			}
			defer func() {
				if err := source.Close(); err != nil {
					t.Error(err)
				}
			}()
			events := make(chan collector.RawEvent, 1)
			if err := source.Run(context.Background(), events); err == nil {
				t.Fatal("invalid record was accepted")
			}
			if len(events) != 0 || source.Stats().ParserErrors != 1 || source.Stats().Received != 1 {
				t.Fatalf("invalid parser counters: %+v", source.Stats())
			}
		})
	}
}

func TestReplayExactRecordLimit(t *testing.T) {
	const maximum = 256
	if len(validRecord)+1 > maximum {
		t.Fatal("test record exceeds boundary")
	}
	exact := validRecord + strings.Repeat(" ", maximum-len(validRecord)-1) + "\n"
	for _, extra := range []int{0, 1} {
		source, err := New(io.NopCloser(strings.NewReader(strings.TrimSuffix(exact, "\n")+strings.Repeat(" ", extra)+"\n")), maximum)
		if err != nil {
			t.Fatal(err)
		}
		events := make(chan collector.RawEvent, 1)
		err = source.Run(context.Background(), events)
		if extra == 0 && (err != nil || len(events) != 1) {
			t.Fatalf("exactly %d bytes must be accepted: %v", maximum, err)
		}
		if extra == 1 && (err == nil || len(events) != 0) {
			t.Fatal("limit plus one byte was accepted")
		}
		if err := source.Close(); err != nil {
			t.Fatal(err)
		}
	}
}

func TestReplayBackpressureAndCancellation(t *testing.T) {
	source, err := New(io.NopCloser(strings.NewReader(validRecord+"\n"+validRecord+"\n")), 65536)
	if err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	events := make(chan collector.RawEvent)
	completed := make(chan error, 1)
	go func() { completed <- source.Run(ctx, events) }()
	select {
	case <-events:
	case <-time.After(time.Second):
		t.Fatal("first record was not delivered")
	}
	select {
	case err := <-completed:
		t.Fatalf("collector did not backpressure: %v", err)
	case <-time.After(20 * time.Millisecond):
	}
	cancel()
	select {
	case err := <-completed:
		if !errors.Is(err, context.Canceled) {
			t.Fatal(err)
		}
	case <-time.After(time.Second):
		t.Fatal("cancellation did not unblock a saturated collector")
	}
	if stats := source.Stats(); stats.Dropped != 1 || stats.Received != 2 {
		t.Fatalf("cancellation loss was not counted: %+v", stats)
	}
	if err := source.Close(); err != nil {
		t.Fatal(err)
	}
}

type brokenInput struct {
	closes int
	err    error
}

func (b *brokenInput) Read([]byte) (int, error) { return 0, b.err }
func (b *brokenInput) Close() error {
	b.closes++
	return b.err
}

func TestReplayReadCloseAndValidationErrors(t *testing.T) {
	sentinel := errors.New("fixture IO failed")
	input := &brokenInput{err: sentinel}
	source, err := New(input, 65536)
	if err != nil {
		t.Fatal(err)
	}
	if err := source.Run(context.Background(), make(chan collector.RawEvent, 1)); !errors.Is(err, sentinel) {
		t.Fatal(err)
	}
	firstClose, secondClose := source.Close(), source.Close()
	if !errors.Is(firstClose, sentinel) || !errors.Is(secondClose, sentinel) || input.closes != 1 {
		t.Fatal("close failure or idempotent ownership was lost")
	}
	if err := source.Run(context.Background(), make(chan collector.RawEvent)); err == nil {
		t.Fatal("closed collector was started")
	}
	for _, size := range []int{255, 1048577} {
		if _, err := New(io.NopCloser(bytes.NewReader(nil)), size); err == nil {
			t.Fatalf("invalid record limit accepted: %d", size)
		}
	}
	if _, err := New(nil, 65536); err == nil {
		t.Fatal("nil reader accepted")
	}
}

func TestReplayExactFieldAndArgumentBounds(t *testing.T) {
	for _, input := range []string{
		strings.Replace(validRecord, `"pid":42`, `"pid":42,"command":"`+strings.Repeat("x", 4096)+`"`, 1),
		strings.Replace(validRecord, `"success":true`, `"success":true,"argv":[`+strings.Repeat(`"x",`, 127)+`"x"]`, 1),
		strings.Replace(validRecord, `"success":true`, `"success":true,"argv":["`+strings.Repeat("x", 4096)+`"]`, 1),
		strings.Replace(validRecord, "2026-01-01T00:00:00.123456789Z", "0001-01-01T00:00:00Z", 1),
	} {
		if _, err := parse([]byte(input), 1); err != nil {
			t.Fatalf("valid boundary observation rejected: %v", err)
		}
	}
}
