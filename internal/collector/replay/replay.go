// SPDX-License-Identifier: Apache-2.0

package replay

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"regexp"
	"strings"
	"sync"
	"sync/atomic"
	"time"
	"unicode/utf8"

	"github.com/blackoutsecure/int-buoybsd/internal/collector"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
)

var timestampFormat = regexp.MustCompile(`^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]{1,9})?(Z|[+-]([01][0-9]|2[0-3]):[0-5][0-9])$`)

type Collector struct {
	input        io.ReadCloser
	maxBytes     int
	started      atomic.Bool
	closed       atomic.Bool
	closeOnce    sync.Once
	closeErr     error
	received     atomic.Uint64
	dropped      atomic.Uint64
	parserErrors atomic.Uint64
}

type record struct {
	Version   string                 `json:"version"`
	Event     string                 `json:"event"`
	Timestamp string                 `json:"timestamp"`
	SourceOS  string                 `json:"source_os"`
	Evidence  string                 `json:"evidence"`
	SourceID  *string                `json:"source_id"`
	Host      *collector.Host        `json:"host"`
	Process   *collector.Process     `json:"process"`
	Jail      *collector.Jail        `json:"jail"`
	Exec      *collector.ProcessExec `json:"exec"`
}

func New(input io.ReadCloser, maxBytes int) (*Collector, error) {
	if input == nil {
		return nil, errors.New("replay input is required")
	}
	if maxBytes < collector.MinRecordBytes || maxBytes > collector.MaxRecordBytes {
		return nil, errors.New("replay max_record_bytes must be between 256 and 1048576")
	}
	return &Collector{input: input, maxBytes: maxBytes}, nil
}

func (c *Collector) Name() string { return "replay" }

func (c *Collector) Stats() collector.Stats {
	return collector.Stats{
		Received:     c.received.Load(),
		Dropped:      c.dropped.Load(),
		ParserErrors: c.parserErrors.Load(),
	}
}

func (c *Collector) Close() error {
	c.closeOnce.Do(func() {
		c.closed.Store(true)
		c.closeErr = c.input.Close()
	})
	return c.closeErr
}

func (c *Collector) Run(ctx context.Context, out chan<- collector.RawEvent) error {
	if c.closed.Load() {
		return errors.New("replay collector is closed")
	}
	if !c.started.CompareAndSwap(false, true) {
		return errors.New("replay collector can only run once")
	}
	reader := bufio.NewReaderSize(c.input, c.maxBytes+1)
	for sequence := uint64(1); ; sequence++ {
		if err := ctx.Err(); err != nil {
			return err
		}
		line, readErr := reader.ReadSlice('\n')
		if len(line) == 0 && errors.Is(readErr, io.EOF) {
			return nil
		}
		if len(line) > c.maxBytes || errors.Is(readErr, bufio.ErrBufferFull) {
			c.received.Add(1)
			c.parserErrors.Add(1)
			return fmt.Errorf("replay record %d exceeds max_record_bytes", sequence)
		}
		if readErr != nil && !errors.Is(readErr, io.EOF) {
			return fmt.Errorf("read replay record %d: %w", sequence, readErr)
		}
		c.received.Add(1)
		raw, err := parse(line, sequence)
		if err != nil {
			c.parserErrors.Add(1)
			return fmt.Errorf("replay record %d: %w", sequence, err)
		}
		select {
		case out <- raw:
		case <-ctx.Done():
			c.dropped.Add(1)
			return ctx.Err()
		}
		if errors.Is(readErr, io.EOF) {
			return nil
		}
	}
}

func parse(line []byte, sequence uint64) (collector.RawEvent, error) {
	if !utf8.Valid(line) {
		return collector.RawEvent{}, errors.New("record is not valid UTF-8")
	}
	if err := uniqueKeys(json.NewDecoder(bytes.NewReader(line)), 0); err != nil {
		return collector.RawEvent{}, err
	}
	var input record
	decoder := json.NewDecoder(bytes.NewReader(line))
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(&input); err != nil {
		return collector.RawEvent{}, errors.New("invalid replay fields or values")
	}
	if err := decoder.Decode(new(any)); !errors.Is(err, io.EOF) {
		return collector.RawEvent{}, errors.New("record must contain exactly one JSON object")
	}
	if input.Version != event.ReplayVersion {
		return collector.RawEvent{}, errors.New("unsupported replay version")
	}
	if input.Event != event.ProcessExec {
		return collector.RawEvent{}, errors.New("unsupported replay event")
	}
	if input.SourceOS != "freebsd" || input.Evidence != "synthetic" {
		return collector.RawEvent{}, errors.New("replay requires source_os freebsd and evidence synthetic")
	}
	if !timestampFormat.MatchString(input.Timestamp) {
		return collector.RawEvent{}, errors.New("timestamp must be RFC3339 with an explicit offset and at most 9 fractional digits")
	}
	timestamp, err := time.Parse(time.RFC3339Nano, input.Timestamp)
	if err != nil || timestamp.UTC().Year() < 1 || timestamp.UTC().Year() > 9999 {
		return collector.RawEvent{}, errors.New("timestamp must be valid and within the protobuf date range")
	}
	if err := validateFields(input); err != nil {
		return collector.RawEvent{}, err
	}
	return collector.RawEvent{
		Name:            input.Event,
		Timestamp:       &timestamp,
		Collector:       "replay",
		OperatingSystem: input.SourceOS,
		Evidence:        input.Evidence,
		SourceID:        input.SourceID,
		Sequence:        sequence,
		Host:            input.Host,
		Process:         input.Process,
		Jail:            input.Jail,
		Exec:            input.Exec,
		Record:          bytes.Clone(line),
		MediaType:       "application/x-buoybsd-replay+json",
	}, nil
}

func validateFields(input record) error {
	fields := []*string{input.SourceID}
	if input.Host != nil {
		fields = append(fields, input.Host.Name)
	}
	if input.Process != nil {
		if input.Process.PID != nil && *input.Process.PID == 0 {
			return errors.New("process-exec PID must be positive when present")
		}
		fields = append(fields, input.Process.ExecutablePath, input.Process.Command)
	}
	if input.Jail != nil {
		fields = append(fields, input.Jail.Name)
	}
	for _, field := range fields {
		if field != nil && len(*field) > collector.MaxFieldBytes {
			return errors.New("record field exceeds 4096 bytes")
		}
	}
	if input.Exec != nil {
		if len(input.Exec.Argv) > collector.MaxArguments {
			return errors.New("argv exceeds 128 entries")
		}
		for _, arg := range input.Exec.Argv {
			if len(arg) > collector.MaxFieldBytes {
				return errors.New("argument exceeds 4096 bytes")
			}
		}
	}
	return nil
}

func uniqueKeys(decoder *json.Decoder, depth int) error {
	if depth > 16 {
		return errors.New("JSON nesting exceeds 16 levels")
	}
	token, err := decoder.Token()
	if err != nil {
		return errors.New("invalid JSON record")
	}
	delim, ok := token.(json.Delim)
	if !ok {
		if depth == 0 {
			return errors.New("record must be a JSON object")
		}
		return nil
	}
	switch delim {
	case '{':
		keys := make(map[string]struct{})
		for decoder.More() {
			key, err := decoder.Token()
			if err != nil {
				return errors.New("invalid JSON object")
			}
			name, ok := key.(string)
			if !ok || name != strings.ToLower(name) {
				return errors.New("invalid JSON field")
			}
			if _, exists := keys[name]; exists {
				return errors.New("duplicate JSON field")
			}
			keys[name] = struct{}{}
			if err := uniqueKeys(decoder, depth+1); err != nil {
				return err
			}
		}
	case '[':
		if depth == 0 {
			return errors.New("record must be a JSON object")
		}
		for decoder.More() {
			if err := uniqueKeys(decoder, depth+1); err != nil {
				return err
			}
		}
	default:
		return errors.New("invalid JSON delimiter")
	}
	if _, err := decoder.Token(); err != nil {
		return errors.New("invalid JSON container")
	}
	return nil
}
