// SPDX-License-Identifier: Apache-2.0

package pipeline

import (
	"context"
	"errors"
	"fmt"
	"slices"

	"github.com/blackoutsecure/int-buoybsd/internal/collector"
	"github.com/blackoutsecure/int-buoybsd/internal/detect"
	"github.com/blackoutsecure/int-buoybsd/internal/normalize"
	"github.com/blackoutsecure/int-buoybsd/internal/output"
	"github.com/blackoutsecure/int-buoybsd/internal/policy"
)

type Options struct {
	QueueSize int
	Policies  *policy.Set
	Detectors []detect.Detector
}

type Stats struct {
	EventsReceived      uint64 `json:"events_received"`
	EventsNormalized    uint64 `json:"events_normalized"`
	EventsFiltered      uint64 `json:"events_filtered"`
	DetectionsGenerated uint64 `json:"detections_generated"`
	EventsDelivered     uint64 `json:"events_delivered"`
	EventsDropped       uint64 `json:"events_dropped"`
	ParserErrors        uint64 `json:"parser_errors"`
	DestinationErrors   uint64 `json:"destination_errors"`
	DeliveryAttempts    uint64 `json:"delivery_attempts"`
	CollectorRestarts   uint64 `json:"collector_restarts"`
}

func Run(parent context.Context, source collector.Collector, destinations []output.Destination, options Options) (stats Stats, result error) {
	if source == nil {
		return stats, errors.New("pipeline requires a collector")
	}
	defer func() {
		if err := source.Close(); err != nil {
			result = errors.Join(result, fmt.Errorf("close collector: %w", err))
		}
		observed := source.Stats()
		stats.EventsReceived = observed.Received
		stats.EventsDropped += observed.Dropped
		stats.ParserErrors = observed.ParserErrors
		stats.CollectorRestarts = observed.Restarts
		for _, destination := range destinations {
			if destination == nil {
				continue
			}
			if err := destination.Close(); err != nil {
				stats.DestinationErrors++
				result = errors.Join(result, fmt.Errorf("close destination %q: %w", destination.Name(), err))
			}
			delivery := destination.Stats()
			stats.DestinationErrors += delivery.Errors
			stats.DeliveryAttempts += delivery.Attempts
		}
	}()
	if options.QueueSize < 1 || options.QueueSize > 4096 || options.Policies == nil || len(destinations) == 0 {
		return stats, errors.New("pipeline requires a bounded queue, compiled policies, and destinations")
	}
	for _, destination := range destinations {
		if destination == nil {
			return stats, errors.New("pipeline destination cannot be nil")
		}
	}
	for _, detector := range options.Detectors {
		if detector == nil {
			return stats, errors.New("pipeline detector cannot be nil")
		}
	}
	ctx, cancel := context.WithCancel(parent)
	defer cancel()
	events := make(chan collector.RawEvent, options.QueueSize)
	completed := make(chan error, 1)
	go func() {
		err := source.Run(ctx, events)
		close(events)
		completed <- err
	}()

processing:
	for {
		if err := ctx.Err(); err != nil {
			result = err
			break
		}
		select {
		case <-ctx.Done():
			result = ctx.Err()
			break processing
		case raw, ok := <-events:
			if !ok {
				break processing
			}
			normalized, err := normalize.Event(raw)
			if err != nil {
				stats.EventsDropped++
				result = err
				break processing
			}
			stats.EventsNormalized++
			retain, matched, err := options.Policies.Match(normalized)
			if err != nil {
				stats.EventsDropped++
				result = fmt.Errorf("evaluate policies: %w", err)
				break processing
			}
			if !retain {
				stats.EventsFiltered++
				continue
			}
			normalized.MatchedPolicies = matched
			for _, detector := range options.Detectors {
				if !slices.Contains(detector.Definition().EventSelectors, normalized.Name) {
					continue
				}
				detection, err := detector.Evaluate(normalized)
				if err != nil {
					stats.EventsDropped++
					result = fmt.Errorf("evaluate detector: %w", err)
					break processing
				}
				if detection != nil {
					normalized.Detections = append(normalized.Detections, detection)
					stats.DetectionsGenerated++
				}
			}
			for _, destination := range destinations {
				if err := destination.WriteEvent(ctx, normalized); err != nil {
					stats.EventsDropped++
					result = fmt.Errorf("deliver to %q: %w", destination.Name(), err)
					break processing
				}
			}
			stats.EventsDelivered++
		}
	}
	cancel()
	collectionErr := <-completed
	// Cancellation caused by a downstream failure must not obscure that failure.
	if !(result != nil && parent.Err() == nil && errors.Is(collectionErr, context.Canceled)) {
		result = errors.Join(result, collectionErr)
	}
	for range events {
		stats.EventsDropped++
	}
	return stats, result
}
