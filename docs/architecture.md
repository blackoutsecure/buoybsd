# Architecture and feasibility assessment

## Decision

Match useful Tracee concepts above the collection boundary; remain
FreeBSD-native below it. Use Go userspace components and an independent
`buoybsd.v1beta1` protobuf contract. This is not a Linux eBPF port or a
wire-compatible reimplementation of Tracee.

```text
FreeBSD telemetry (future) / replay envelope (implemented)
  -> collector -> bounded raw-event queue
  -> normalization -> observed context / incomplete-data warnings
  -> policy filtering -> native Go detections
  -> one default broadcast stream -> destinations
```

The replay slice implements process execution only. No stage consults
the development host to invent FreeBSD context. A future enricher must
deal explicitly with PID reuse, process lifetime, and lookup races.

## Boundaries

- `api/v1beta1`: generated, externally versioned event messages.
- `cmd/buoybsd`, `cmd/buoyctl`: thin command entry points.
- `internal/collector`: backend-neutral observations and counters.
- `internal/collector/replay`: strict bounded replay-envelope decoding.
- `internal/normalize`: observed values to protobuf, lossless raw bytes.
- `internal/policy`: validated exact/glob selectors.
- `internal/detect`: informational shell-execution annotation.
- `internal/pipeline`: cancellation, backpressure, lifecycle, metrics.
- `internal/output`: JSON/table formats and webhook delivery.
- `internal/cli`, `pkg/config`: flags and one typed YAML configuration.
- `pkg/event`: event definitions, identifiers, and schema constants.
- `tools/dev`: portable build/format/generation/license checks.

Do not create empty native collector, service, stream, or API packages.
The control client lists the compiled event catalogue and version.
`status` explicitly fails because there is no running control service.

## Runtime and failure model

One collector goroutine feeds a fixed-capacity channel. One consumer
processes events in replay order and broadcasts each accepted event
synchronously to all configured destinations, in configuration order.
Normal replay blocks when the queue is full. There is no unbounded
destination queue, collector restart loop, or implicit event sampling.

Parser, normalization, detector, and destination failures stop the run.
Cancellation stops collection; queued and unsent observations are
counted as dropped. All owned resources are closed and close failures
are returned. A generic blocked writer cannot be forcibly interrupted;
HTTP writes have explicit timeouts.

Delivery is not transactional. A later destination can fail after an
earlier one accepted the event. Webhook retry can duplicate an event.
The record hash supports correlation, not globally unique occurrence
identity or receiver-side deduplication guarantees.

## Phasing

1. **Implemented:** synthetic replay contract, protobuf normalization,
   policy/detector, JSON/file/table/webhook, tests, metrics, tooling.
2. **Next experiment:** manually inspect FreeBSD DTrace `proc` probes
   and audit facilities in an isolated, supported FreeBSD VM.
3. **Evidence gate:** real sanitized captures and semantic review.
4. **Next implementation:** one native process-exec collector behind a
   FreeBSD build tag, with explicit privileges/loss reporting.
5. **Only afterward:** process exit, file access, outbound connection,
   bind, credential change; native jail/Capsicum/pf/module events.
6. **Deferred:** Go templates, Forward, gRPC, multiple streams, Rego,
   persistent management, and remote control.

The first prototype is useful for contract development; it is not
evidence of native acquisition feasibility or production suitability.
