# Architecture and feasibility assessment

## Decision

Match useful Tracee concepts above the collection boundary; remain
FreeBSD-native below it. Use Rust userspace components and an independent
`buoybsd.v1beta1` protobuf contract. This is not a Linux eBPF port or a
wire-compatible reimplementation of Tracee.

```text
FreeBSD telemetry (future) / replay envelope (implemented)
  -> collector -> bounded raw-event queue
  -> normalization -> observed context / incomplete-data warnings
  -> policy filtering -> pure-Rust detections
  -> one default broadcast stream -> destinations
```

The replay slice implements process execution only. No stage consults
the development host to invent FreeBSD context. A future enricher must
deal explicitly with PID reuse, process lifetime, and lookup races.

## Boundaries

- `api/v1beta1`: unchanged schema and generated Prost/protobuf-JSON types.
- `src/bin`: thin `buoybsd`, `buoyctl`, and opt-in developer commands.
- `src/domain.rs`, `src/glob.rs`, `src/time.rs`: pure-Rust observations,
  events, presence, catalogue, policies, detectors, and timestamp rules.
  Only narrow Serde/thiserror support crosses this domain boundary.
- `src/replay.rs`: strict bounded replay acquisition and counters.
- `src/normalize.rs`: source-preserving normalization and content hash.
- `src/wire.rs`: protobuf and canonical JSON adapters; no protobuf
  runtime is imported into domain code.
- `src/pipeline.rs`: Tokio cancellation, backpressure, lifecycle, metrics.
- `src/output.rs`, `src/io.rs`: JSON/table delivery, Reqwest webhooks,
  append-only files, and deterministic OS-handle closure.
- `src/cli.rs`, `src/config.rs`, `src/diagnostics.rs`: Clap, typed YAML
  validation, and Tracing diagnostics isolated from event stdout.
- `src/dev.rs`: feature-gated generation/license/cleanup tooling.
- `tests`: migrated regressions, subprocess checks, and replay integration.

This is one small Cargo package, not an empty layered workspace.
There is no HTTP server, database, Go runtime/fallback, or UI.
Tokio exists for bounded asynchronous acquisition, cancellation,
signals, and HTTP, not for pure policy/detector evaluation.

Do not create empty native collector, service, stream, or API packages.
The control client lists the compiled event catalogue and version.
`status` explicitly fails because there is no running control service.

## Runtime and failure model

One collector task feeds a fixed-capacity Tokio channel. One consumer
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
   FreeBSD `cfg`, with explicit privileges/loss reporting.
5. **Only afterward:** process exit, file access, outbound connection,
   bind, credential change; native jail/Capsicum/pf/module events.
6. **Deferred:** templates, Forward, gRPC, multiple streams, Rego,
   persistent management, and remote control.

The first prototype is useful for contract development; it is not
evidence of native acquisition feasibility or production suitability.
