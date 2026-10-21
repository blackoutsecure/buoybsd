# Limitations and completion gates

## Present limitations

- No live collector, production sensor, native audit parser, or tested
  DTrace script. No FreeBSD release is runtime-certified.
- Synthetic examples validate the replay/API contract only.
- Process execution is the only supported event.
- No live enrichment, process ancestry cache, durable spool, rotation,
  automatic restart, retention, or remote control API.
- One bounded default stream; synchronous destinations share
  backpressure. A failed output stops all processing.
- Webhooks can duplicate delivery during retry. Files are appended,
  not fsynced per event. Delivery counters do not prove durability.
- Generic writer calls cannot be forcibly cancelled while inside an
  operating-system write; webhook requests have finite timeouts.
- A raw record may contain sensitive arguments. There is no automatic
  redaction of the preserved original bytes.
- JSON is BuoyBSD protobuf JSON, not Tracee JSON. Compatibility is
  unverified.
- Field/record/queue bounds are explicit; oversized input fails rather
  than truncating unnoticed. Queue capacity times record size can still
  be significant; tune both, not just one.
- UID/GID selectors use real IDs. Network/severity selectors, templates,
  Forward, gRPC, Rego, and multiple streams are deferred.
- Windows race validation was attempted but blocked by ThreadSanitizer
  shadow-memory allocation error 87 before tests executed. The Linux
  CI race gate is configured but has not run against this unpublished
  bootstrap. Ordinary local tests are not a substitute for that gate.

## Replay bootstrap gate

The bootstrap must provide the architecture/feasibility documents,
compatibility matrix, limitations, proposed protobuf, Go skeleton,
synthetic replay collector, JSON output and tested replay-to-JSON path.
It additionally exercises policy, detector, table/file/webhook,
cancellation, bounded buffering, configuration, and diagnostics.

## Native process-execution POC gate: not yet satisfied

On a currently supported FreeBSD system, demonstrate:

1. Real process-execution capture with a documented privilege model.
2. Exact raw record preservation and captured-fixture provenance.
3. `v1beta1` normalization and reliable observed process identity.
4. Explicit unknown/truncated/race-prone fields.
5. Policy filtering and at least one detector with source reference.
6. JSON, safe table output, and test-webhook delivery.
7. Parser/error/drop counters, including backend/kernel loss.
8. Unit tests and explicitly enabled native integration tests.
9. FreeBSD version/architecture and performance evidence.
10. Reviewed Tracee field mapping and semantic differences.

Do not call a cross-compiled executable a successful native runtime
test. Do not weaken these gates because a FreeBSD runner is unavailable.
The next action is the small manual experiment in
[the telemetry assessment](freebsd-telemetry.md), not broad collector
implementation.
