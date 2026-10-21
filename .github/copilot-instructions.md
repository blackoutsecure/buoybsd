# BuoyBSD Copilot instructions

## Identity and scope

BuoyBSD is an independent experimental FreeBSD runtime-security,
observability, and forensics project. The existing repository is
`blackoutsecure/int-buoybsd`; commands are `buoybsd` and `buoyctl`.
Never claim affiliation with Aqua Security, Tracee, or the FreeBSD
Project, or compatibility unsupported by tests and semantic review.

Use [the architecture](../docs/architecture.md) and
[completion gates](../docs/limitations.md) as the scope contract.
The initial assignment is a replay prototype and feasibility study,
not permission to build all live collectors at once. Finish the
process-execution vertical slice before expanding.

## Language and architecture

- Use Go for commands, API integration, collectors, normalization,
  enrichment, policies, detection, routing, outputs, and tests.
- Use FreeBSD-native acquisition. Investigate auditpipe, DTrace,
  kqueue, devd, sysctl, socket/routing interfaces, pf, jails, and
  Capsicum only where a concrete telemetry requirement justifies them.
- Use C/cgo only when a safe, maintainable Go interface is not practical.
  Isolate it behind a small Go interface and document why it is needed.
- Keep acquisition separate from normalization, enrichment, filtering,
  detection, routing, and serialization. Do not put detections inside
  collectors or native backend details inside the pipeline.
- Native implementations need `//go:build freebsd`. Other platforms
  use replay/test doubles; never pretend a stub is a live sensor.
- Do not import libbpf, Linux eBPF, BTF/CO-RE, cgroups, namespaces, LSM,
  Linux syscall numbers, Kubernetes context, or `/proc` assumptions.
  Jail IDs are not Linux namespace or container IDs.
- Create packages only when behavior exists. Do not populate empty
  hierarchies or prematurely add multiple streams, Rego, or a remote
  management plane.

## Events and compatibility

The external contract is [event.proto](../api/v1beta1/event.proto);
the rules and replay format are in
[the event-model document](../docs/event-model.md).

- Keep original raw records and source identifiers separate from
  normalized fields. Never discard an unmapped source value.
- Preserve observed timestamps and ordering information; do not promise
  global ordering across independent sources.
- Use protobuf field presence for unavailable scalars. Observed UID 0
  is different from an absent UID. Never fabricate missing context,
  ancestry, executable paths, or successful outcomes.
- Keep host, process, real/effective credentials, thread, executable,
  jail, typed data, policies, detections, source references, schema
  version, and warnings explicit as they become reliably available.
- Do not renumber protobuf fields or reuse removed numbers. Keep
  BuoyBSD IDs distinct from Tracee/Linux IDs.
- Update [the compatibility matrix](../docs/tracee-compatibility.md)
  for every mapping: both names/IDs, source, fields, gaps, privileges,
  OS versions, test evidence, and semantic review.
- Status vocabulary: `equivalent`, `partially-equivalent`,
  `FreeBSD-specific`, `unsupported`, `under-investigation`.
  Only automated evidence plus semantic review permits `equivalent`.
- Treat synthetic replay examples as synthetic. Native parser fixtures
  must be sanitized real captures with provenance, capture command,
  OS/kernel version, truncation/drop information, and source format.
  Never manufacture audit or DTrace output and call it captured data.

## Collection and reliability

Collectors must honor cancellation, close resources deterministically,
return actionable errors, identify their source, preserve raw bytes,
and expose event-loss/parser counters. All queues and allocations must
be bounded. Replay should backpressure, not silently drop data.

Document each native collector's supported versions, privileges,
preselection, event loss, semantic gaps, performance, and shutdown.
Track received, normalized, filtered, detected, delivered, and dropped
events, parser/destination errors, and collector restarts. Surface
overflow, failed delivery, and partial data explicitly.

Enrichment must be race-aware: a later PID lookup is not proof of the
original process identity. Do not enrich a FreeBSD replay from the
developer's unrelated host.

## Policies, detections, and outputs

- Use typed YAML configuration and strict unknown-key validation.
  Validate before starting collectors; reject unsupported features,
  malformed URLs, excessive sizes, and contradictory settings.
- Start with exact matching and documented full-path globs. Within one
  policy, selectors are ANDed; values and policies are ORed. Missing
  observed fields never satisfy a numeric/path selector.
- Use small native Go detectors first, declaring stable ID, name,
  description, selectors, severity, tags, required fields, and an
  evaluation function. Detection annotations retain source references.
  Do not call ordinary shell execution proof of a threat.
- Separate formats from delivery. Initial output order is JSON stdout,
  JSON file, escaped table, webhook, then (only after evidence) Go
  templates, Fluent Forward, and gRPC.
- Start with one default stream. Multiple streams and severity/network
  selectors are future work until relevant event types exist.
- Separate diagnostic logs from event output. JSON field semantics
  must stay documented and machine-readable; protobuf JSON is not
  Tracee JSON merely because both use a `v1beta1` package.
- Webhooks require bounded timeouts, retries/buffering, TLS verification,
  observable failures, secret-safe custom headers, and explicit
  duplicate-delivery semantics. Never follow redirects with secrets.
- Local runtime control, if justified, uses a permission-restricted
  Unix-domain socket. Remote network control requires a separate design
  and security review. Do not invent a working status service.

## Security

Follow [the security model](../docs/security-model.md).
Treat telemetry as hostile: control characters, invalid UTF-8, long
paths, malformed numbers/addresses, and hostile arguments are expected.
Bound record/field sizes, escape terminal output, and avoid shell
invocation. Never build commands from event values.

Never log environments, webhook header values, secret-bearing URLs,
or full raw events by default. Use least privilege, document every
required privilege, and keep privileged/native tests opt-in. Never run
destructive host tests or silently change audit/DTrace configuration.

Use normal FreeBSD filesystem conventions. Verify paths against
FreeBSD documentation rather than guessing from Linux.

## Build, tests, and documentation

See [CONTRIBUTING.md](../CONTRIBUTING.md) for exact commands and pinned
tool versions. Keep Make and direct Go commands consistent.

- Add table-driven tests for parsing, normalization, missing fields,
  policy matching, detectors, JSON, failures, saturation, cancellation,
  and configuration validation.
- Add replay integration coverage for every supported vertical slice.
  FreeBSD runtime tests require build tags and an explicit opt-in when
  privileged. A skip or cross-build is never a passed native test.
- Maintain architecture, event model, telemetry feasibility,
  compatibility, limitations, security model, and provenance documents.
  Keep README, CONTRIBUTING, SECURITY, NOTICE, and configuration current.
- Generated protobuf code must be reproducible using the recorded
  command/tool versions. Run generation consistency checks.
- CI must distinguish formatting/static analysis, unit/race tests,
  replay integration, FreeBSD compilation, and native privileged tests.
  Pin remote step actions to full SHAs with version comments.

## Licensing and upstream research

Tracee reference: <https://github.com/aquasecurity/tracee>.
Its pipeline, API, policies, detectors, signatures, streams, outputs,
CLI, and testing are references, not a mandate to copy Linux code.
Use the pinned evidence in [provenance](../docs/provenance.md).

Original code uses Apache-2.0. Before copying/adapting anything, inspect
the exact file, directory, LICENSE, and NOTICE; preserve notices; record
the source ref, whether copied/modified/derived/independent, and changes.
Do not assume Tracee's entire tree has one license, copy its eBPF/C
layer, remove attribution, or present uncertain legal conclusions as
fact. Flag legal uncertainty for human review.

## Change discipline and next work

Keep changes small and vertical. PRs state the problem, telemetry source,
security/privilege implications, exact tests/results, compatibility
impact, and limitations. Use scoped conventional, imperative subjects;
do not invent issue references.

Next: perform only the smallest manual FreeBSD process-exec experiment
in [the feasibility assessment](../docs/freebsd-telemetry.md), capture
real evidence, review it, and propose a collector. Live POC completion
requires real collection, raw preservation, normalization, observed
identity, a policy/detector, JSON/table/webhook delivery, loss/error
counters, FreeBSD integration tests, and documented semantic gaps.
Do not expand to exit/file/network/credential/jail/Capsicum/pf/module
events until that slice is reproducible.
