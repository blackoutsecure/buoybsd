# Event model

## Version and identity

The authoritative schema is
[`api/v1beta1/event.proto`](../api/v1beta1/event.proto).
Its protobuf package is `buoybsd.v1beta1`; its JSON schema marker is
`buoybsd.v1beta1`. This schema is experimental and not Tracee wire JSON.
Never reuse protobuf field numbers after removal.

Process execution is `process_exec`, ID **1000001**. BuoyBSD reserves
1000000-1999999 for its own normalized events; these are not Linux
syscall numbers or copied Tracee IDs.

`record_id` is `sha256:` plus the hex digest of the preserved raw
record, including its input line ending. Identical input bytes produce
the same ID, including across replays; it is a content reference, not
a unique occurrence ID. `source.replay_sequence` is one-based input
order, not a kernel timestamp or global sequence.

The source timestamp is mandatory, RFC3339 with an explicit offset,
and normalized without replacing it with the ingestion clock.
Protobuf timestamps have nanosecond precision; more than nine fractional
digits, invalid offsets, and dates outside the protobuf range are
rejected, not silently truncated. An observed year-one timestamp is
distinct from a missing timestamp. JSON uses protobuf's canonical UTC
timestamp formatting.

## Presence and source fidelity

Optional process IDs, credentials, executable path, command, host,
jail ID/name, source ID, and argument-completeness values distinguish
unavailable data from observed zero/empty values. Missing argv and
`argv_complete: false` must not be called a complete argument vector.
UID/GID 0 is valid observed data; PID 0 is not accepted for a user
process-exec observation.

An absent process or executable path produces an explicit warning.
No ancestry, network, Linux container, Kubernetes, or syscall fields
are fabricated. Future typed event messages can add those concepts
only when evidence exists.

`raw_record.data` contains exact input bytes as protobuf `bytes`
(base64 in JSON). Its media type identifies a BuoyBSD replay envelope,
not native BSM or DTrace output. Normalization retains all supported
raw values. Unknown fields are rejected rather than silently lost.

Detections annotate an event and contain a source reference to its
name, numeric ID, and raw-record hash. They do not replace the source
event or claim that an informational match is malicious behavior.

## Replay envelope v1

One UTF-8 JSON object per line. A final line without newline is valid;
blank lines, duplicate object keys, unknown fields, invalid UTF-8,
multiple JSON values, and unsupported versions/events are errors.
The default maximum line size, **including its line ending**, is
65536 bytes; allowed configuration range is 256-1048576 bytes.
Field strings are limited to 4096 bytes and argv to 128 entries.
Queue capacity is 1-4096 records, and capacity times the configured raw
record limit must not exceed 64 MiB. This is a raw queue budget, not a
promise of exact total process memory consumption.

```json
{
  "version": "buoybsd.replay.v1",
  "event": "process_exec",
  "timestamp": "2026-01-01T00:00:00.123456789Z",
  "source_os": "freebsd",
  "evidence": "synthetic",
  "source_id": "synthetic-exec-1",
  "host": {"name": "fixture-host"},
  "process": {
    "pid": 4242,
    "ppid": 4000,
    "uid": 0,
    "euid": 0,
    "gid": 0,
    "egid": 0,
    "executable_path": "/bin/sh",
    "command": "sh"
  },
  "exec": {"argv": ["sh", "-c", "true"], "argv_complete": true}
}
```

The current reader accepts **synthetic** evidence only. That restriction
prevents an unverified replay document from being represented as a
validated native capture. Real telemetry requires a source-specific
parser, capture manifest, and semantic review first.

Optional jail context is `{"id": 0, "name": "host"}` when actually
observed, not a default invented from a missing field.

## Policies

Policy names are unique. Within a policy, selector categories are
ANDed; values within a category are ORed. Policies are ORed. No policies
means retain all events. Missing fields do not match selectors,
including UID/GID/jail ID 0. All matching names are retained.

Implemented selectors: `event_names`, `collectors`, `executables`,
`uids`, `gids`, `jail_ids`, and `process_names`.
UID/GID selectors use **real**, not effective, IDs. Executables and
process names use Go `path.Match`: full, case-sensitive matching;
`*` does not cross `/`, and `**` has no recursive-directory semantics.
An exact string without metacharacters is an exact match.

Network/severity selectors and compound expression languages are not
implemented. Unknown YAML fields fail instead of doing nothing.

## JSON and metrics

JSON uses protobuf field names (snake_case), one object per line.
Absent fields are omitted. Protobuf JSON represents 64-bit integers as
strings, enum values as names, bytes as base64, and timestamps as
RFC3339 strings. Object-key order and byte-for-byte serialization are
not an API guarantee; field semantics are the contract.

Final structured stderr diagnostics contain: `events_received`,
`events_normalized`, `events_filtered`, `detections_generated`,
`events_delivered`, `events_dropped`, `parser_errors`,
`destination_errors`, and `collector_restarts`.
Received counts nonempty/invalid input records attempted, including a
failing record. Delivered means accepted by **every** configured
destination, not durable remote storage. Drops include cancellation,
failed in-flight delivery, normalization failure, and queued abandoned
records. Parser failures are reported separately. Replay never
automatically restarts.
