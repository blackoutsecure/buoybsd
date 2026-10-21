// SPDX-License-Identifier: Apache-2.0

package pipeline

import (
	"context"
	"errors"
	"io"
	"strings"
	"sync/atomic"
	"testing"
	"time"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"github.com/blackoutsecure/int-buoybsd/internal/collector"
	"github.com/blackoutsecure/int-buoybsd/internal/collector/replay"
	"github.com/blackoutsecure/int-buoybsd/internal/detect"
	"github.com/blackoutsecure/int-buoybsd/internal/output"
	"github.com/blackoutsecure/int-buoybsd/internal/policy"
	"github.com/blackoutsecure/int-buoybsd/pkg/config"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
)

func ptr[T any](value T) *T { return &value }

type testCollector struct {
	events   []collector.RawEvent
	runErr   error
	closeErr error
	received atomic.Uint64
	dropped  atomic.Uint64
	closes   atomic.Uint64
}

func (c *testCollector) Name() string { return "test-replay" }
func (c *testCollector) Close() error {
	c.closes.Add(1)
	return c.closeErr
}
func (c *testCollector) Stats() collector.Stats {
	return collector.Stats{Received: c.received.Load(), Dropped: c.dropped.Load()}
}
func (c *testCollector) Run(ctx context.Context, out chan<- collector.RawEvent) error {
	for _, raw := range c.events {
		if err := ctx.Err(); err != nil {
			return err
		}
		c.received.Add(1)
		select {
		case out <- raw:
		case <-ctx.Done():
			c.dropped.Add(1)
			return ctx.Err()
		}
	}
	return c.runErr
}

func observation() collector.RawEvent {
	return collector.RawEvent{
		Name: event.ProcessExec, Timestamp: ptr(time.Unix(1700000000, 0)),
		Collector: "replay", OperatingSystem: "freebsd", Evidence: "synthetic",
		Record: []byte("synthetic observation"), MediaType: "application/x-buoybsd-replay+json",
		Process: &collector.Process{PID: ptr(uint32(42)), UID: ptr(uint32(0)), ExecutablePath: ptr("/bin/sh")},
		Exec:    &collector.ProcessExec{Success: ptr(true)},
	}
}

func options(t *testing.T, policies []config.Policy) Options {
	t.Helper()
	set, err := policy.Compile(policies)
	if err != nil {
		t.Fatal(err)
	}
	return Options{QueueSize: 1, Policies: set, Detectors: []detect.Detector{detect.ShellExec{}}}
}

func jsonDestination(t *testing.T, writer io.Writer, closer io.Closer) output.Destination {
	t.Helper()
	destination, err := output.NewJSON("json", writer, closer)
	if err != nil {
		t.Fatal(err)
	}
	return destination
}

func TestPipelineFilteringDetectionAndMetrics(t *testing.T) {
	second := observation()
	second.Process.UID = ptr(uint32(1000))
	source := &testCollector{events: []collector.RawEvent{observation(), second}}
	stats, err := Run(context.Background(), source, []output.Destination{jsonDestination(t, io.Discard, nil)},
		options(t, []config.Policy{{Name: "root", UIDs: []uint32{0}}}))
	if err != nil {
		t.Fatal(err)
	}
	if stats != (Stats{EventsReceived: 2, EventsNormalized: 2, EventsFiltered: 1,
		DetectionsGenerated: 1, EventsDelivered: 1, DeliveryAttempts: 1}) {
		t.Fatalf("incorrect metrics: %+v", stats)
	}
	if source.closes.Load() != 1 {
		t.Fatal("collector ownership was not closed exactly once")
	}
}

type failedWriter struct{ err error }

func (w failedWriter) Write([]byte) (int, error) { return 0, w.err }

func TestPipelineOutputFailureStopsAndAccountsForAbandonedEvents(t *testing.T) {
	sentinel := errors.New("fixture destination failure")
	source := &testCollector{}
	for range 100 {
		source.events = append(source.events, observation())
	}
	stats, err := Run(context.Background(), source,
		[]output.Destination{jsonDestination(t, failedWriter{sentinel}, nil)}, options(t, nil))
	if !errors.Is(err, sentinel) {
		t.Fatal(err)
	}
	if stats.EventsReceived == 0 || stats.EventsReceived > 3 || stats.EventsDelivered != 0 ||
		stats.EventsDropped != stats.EventsReceived || stats.DestinationErrors != 1 || source.closes.Load() != 1 {
		t.Fatalf("bounded failure/loss accounting failed: %+v", stats)
	}
}

type blockingDestination struct {
	entered  chan struct{}
	attempts atomic.Uint64
	failures atomic.Uint64
	closes   atomic.Uint64
}

func (b *blockingDestination) Name() string { return "blocking-fixture" }
func (b *blockingDestination) WriteEvent(ctx context.Context, _ *v1.Event) error {
	b.attempts.Add(1)
	b.entered <- struct{}{}
	<-ctx.Done()
	b.failures.Add(1)
	return ctx.Err()
}
func (b *blockingDestination) Close() error {
	b.closes.Add(1)
	return nil
}
func (b *blockingDestination) Stats() output.Stats {
	return output.Stats{Attempts: b.attempts.Load(), Errors: b.failures.Load()}
}

func TestPipelineCancellationUnblocksSaturatedQueue(t *testing.T) {
	source := &testCollector{}
	for range 1000 {
		source.events = append(source.events, observation())
	}
	destination := &blockingDestination{entered: make(chan struct{}, 1)}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	type result struct {
		stats Stats
		err   error
	}
	done := make(chan result, 1)
	opts := options(t, nil)
	go func() {
		stats, err := Run(ctx, source, []output.Destination{destination}, opts)
		done <- result{stats, err}
	}()
	select {
	case <-destination.entered:
	case <-time.After(time.Second):
		t.Fatal("output was not reached")
	}
	if source.Stats().Received > 3 {
		t.Fatal("bounded queue was bypassed")
	}
	cancel()
	select {
	case result := <-done:
		if !errors.Is(result.err, context.Canceled) || result.stats.EventsDelivered != 0 ||
			result.stats.EventsDropped != result.stats.EventsReceived ||
			source.closes.Load() != 1 || destination.closes.Load() != 1 {
			t.Fatalf("cancellation did not clean up/count loss: %+v %v", result.stats, result.err)
		}
	case <-time.After(time.Second):
		t.Fatal("pipeline goroutine leaked after cancellation")
	}
}

type failedCloser struct{ err error }

func (c failedCloser) Close() error { return c.err }

type failedDetector struct{ err error }

func (d failedDetector) Definition() detect.Definition {
	return detect.Definition{ID: "fixture", EventSelectors: []string{event.ProcessExec}}
}
func (d failedDetector) Evaluate(*v1.Event) (*v1.Detection, error) { return nil, d.err }

func TestPipelinePropagatesCollectionNormalizationDetectionAndCloseFailures(t *testing.T) {
	sentinel := errors.New("fixture failure")
	t.Run("collection", func(t *testing.T) {
		source := &testCollector{events: []collector.RawEvent{observation()}, runErr: sentinel}
		stats, err := Run(context.Background(), source,
			[]output.Destination{jsonDestination(t, io.Discard, nil)}, options(t, nil))
		if !errors.Is(err, sentinel) || stats.EventsDelivered != 1 {
			t.Fatalf("collection failure hidden: %+v %v", stats, err)
		}
	})
	t.Run("normalization", func(t *testing.T) {
		raw := observation()
		raw.Name = "unknown"
		source := &testCollector{events: []collector.RawEvent{raw}}
		stats, err := Run(context.Background(), source,
			[]output.Destination{jsonDestination(t, io.Discard, nil)}, options(t, nil))
		if err == nil || stats.EventsDropped != 1 || stats.EventsNormalized != 0 {
			t.Fatalf("normalization failure hidden: %+v %v", stats, err)
		}
	})
	t.Run("detector", func(t *testing.T) {
		opts := options(t, nil)
		opts.Detectors = []detect.Detector{failedDetector{sentinel}}
		source := &testCollector{events: []collector.RawEvent{observation()}}
		stats, err := Run(context.Background(), source,
			[]output.Destination{jsonDestination(t, io.Discard, nil)}, opts)
		if !errors.Is(err, sentinel) || stats.EventsDropped != 1 {
			t.Fatalf("detector failure hidden: %+v %v", stats, err)
		}
	})
	t.Run("close", func(t *testing.T) {
		source := &testCollector{closeErr: sentinel}
		stats, err := Run(context.Background(), source,
			[]output.Destination{jsonDestination(t, io.Discard, failedCloser{sentinel})}, options(t, nil))
		if !errors.Is(err, sentinel) || stats.DestinationErrors != 1 {
			t.Fatalf("close failures hidden: %+v %v", stats, err)
		}
	})
	t.Run("parser", func(t *testing.T) {
		source, err := replay.New(io.NopCloser(strings.NewReader("{}\n")), 65536)
		if err != nil {
			t.Fatal(err)
		}
		stats, err := Run(context.Background(), source,
			[]output.Destination{jsonDestination(t, io.Discard, nil)}, options(t, nil))
		if err == nil || stats.ParserErrors != 1 || stats.EventsReceived != 1 || stats.EventsNormalized != 0 {
			t.Fatalf("parser failure hidden: %+v %v", stats, err)
		}
	})
}

func TestPipelinePartialBroadcastIsNotReportedDelivered(t *testing.T) {
	sentinel := errors.New("fixture second destination failure")
	source := &testCollector{events: []collector.RawEvent{observation()}}
	stats, err := Run(context.Background(), source, []output.Destination{
		jsonDestination(t, io.Discard, nil), jsonDestination(t, failedWriter{sentinel}, nil),
	}, options(t, nil))
	if !errors.Is(err, sentinel) || stats.DeliveryAttempts != 2 || stats.EventsDelivered != 0 ||
		stats.EventsDropped != 1 || stats.DestinationErrors != 1 {
		t.Fatalf("partial broadcast was misreported: %+v %v", stats, err)
	}
}
