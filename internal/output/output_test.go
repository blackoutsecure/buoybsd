// SPDX-License-Identifier: Apache-2.0

package output

import (
	"bytes"
	"context"
	"errors"
	"io"
	"strings"
	"testing"
	"time"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
	"google.golang.org/protobuf/encoding/protojson"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func ptr[T any](value T) *T { return &value }

func sampleEvent() *v1.Event {
	return &v1.Event{
		Timestamp: timestamppb.New(time.Unix(1700000000, 123)),
		Id:        v1.EventId_EVENT_ID_PROCESS_EXEC, Name: event.ProcessExec,
		SchemaVersion: event.SchemaVersion, RecordId: "sha256:synthetic",
		Process: &v1.Process{Pid: ptr(uint32(42)), Uid: ptr(uint32(0)), ExecutablePath: ptr("/bin/sh")},
		Source:  &v1.Source{Collector: "replay", Evidence: "synthetic", ReplaySequence: 1},
		Data:    &v1.Event_ProcessExec{ProcessExec: &v1.ProcessExec{Success: ptr(true)}},
		RawRecord: &v1.RawRecord{
			MediaType: "application/x-buoybsd-replay+json", Data: []byte("synthetic\r\n"),
		},
	}
}

func TestJSONRoundTripAndLineContract(t *testing.T) {
	var buffer bytes.Buffer
	destination, err := NewJSON("json", &buffer, nil)
	if err != nil {
		t.Fatal(err)
	}
	for range 2 {
		if err := destination.WriteEvent(context.Background(), sampleEvent()); err != nil {
			t.Fatal(err)
		}
	}
	lines := bytes.Split(bytes.TrimSuffix(buffer.Bytes(), []byte("\n")), []byte("\n"))
	if len(lines) != 2 {
		t.Fatal("JSON did not produce one complete object per line")
	}
	for _, line := range lines {
		var decoded v1.Event
		if err := protojson.Unmarshal(line, &decoded); err != nil {
			t.Fatal(err)
		}
		if !proto.Equal(sampleEvent(), &decoded) {
			t.Fatal("protobuf presence or raw bytes did not survive JSON")
		}
	}
	if destination.Stats() != (Stats{Attempts: 2}) {
		t.Fatal(destination.Stats())
	}
}

type brokenWriter struct{ err error }

func (b brokenWriter) Write([]byte) (int, error) { return 0, b.err }

type shortWriter struct{}

func (shortWriter) Write(data []byte) (int, error) { return len(data) - 1, nil }

type brokenCloser struct{ err error }

func (b brokenCloser) Close() error { return b.err }

func TestJSONFailureAndCancellation(t *testing.T) {
	sentinel := errors.New("fixture write failure")
	for _, test := range []struct {
		writer io.Writer
		err    error
	}{{brokenWriter{sentinel}, sentinel}, {shortWriter{}, io.ErrShortWrite}} {
		destination, err := NewJSON("json", test.writer, nil)
		if err != nil {
			t.Fatal(err)
		}
		if err := destination.WriteEvent(context.Background(), sampleEvent()); !errors.Is(err, test.err) {
			t.Fatal(err)
		}
		if destination.Stats() != (Stats{Attempts: 1, Errors: 1}) {
			t.Fatal(destination.Stats())
		}
	}
	destination, err := NewJSON("json", io.Discard, brokenCloser{sentinel})
	if err != nil {
		t.Fatal(err)
	}
	if !errors.Is(destination.Close(), sentinel) {
		t.Fatal("close failure was hidden")
	}
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	if err := destination.WriteEvent(ctx, sampleEvent()); !errors.Is(err, context.Canceled) {
		t.Fatal(err)
	}
	if err := destination.WriteEvent(context.Background(), nil); err == nil {
		t.Fatal("nil event accepted")
	}
	if _, err := NewJSON("json", nil, nil); err == nil {
		t.Fatal("nil writer accepted")
	}
}

func TestTableEscapesTerminalData(t *testing.T) {
	source := sampleEvent()
	source.Name = "exec\x1b[31m\n"
	source.Process.ExecutablePath = ptr("/bin/hostile\x1b[2J\n\t\u202e")
	var buffer bytes.Buffer
	destination, err := NewTable("table", &buffer)
	if err != nil {
		t.Fatal(err)
	}
	for range 2 {
		if err := destination.WriteEvent(context.Background(), source); err != nil {
			t.Fatal(err)
		}
	}
	text := buffer.String()
	if strings.ContainsRune(text, '\x1b') || strings.ContainsRune(text, '\u202e') ||
		strings.Count(text, "\n") != 3 || strings.Count(text, "TIME\tEVENT") != 1 ||
		!strings.Contains(text, `\x1b`) || !strings.Contains(text, `\n`) {
		t.Fatalf("unsafe or malformed table: %q", text)
	}
	if err := destination.Close(); err != nil {
		t.Fatal(err)
	}
}

func TestTableRejectsInvalidEventsAndWrites(t *testing.T) {
	destination, err := NewTable("table", brokenWriter{io.ErrClosedPipe})
	if err != nil {
		t.Fatal(err)
	}
	if err := destination.WriteEvent(context.Background(), sampleEvent()); !errors.Is(err, io.ErrClosedPipe) {
		t.Fatal(err)
	}
	for _, source := range []*v1.Event{nil, {}, {Timestamp: &timestamppb.Timestamp{Seconds: 253402300800}}} {
		destination, err := NewTable("table", io.Discard)
		if err != nil {
			t.Fatal(err)
		}
		if err := destination.WriteEvent(context.Background(), source); err == nil {
			t.Fatal("invalid table input accepted")
		}
	}
	if _, err := NewTable("table", nil); err == nil {
		t.Fatal("nil writer accepted")
	}
}
