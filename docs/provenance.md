# Licensing and provenance

## Original implementation

BuoyBSD's Go code, protobuf schema, configuration, documentation, and
synthetic replay envelopes are independently implemented for this
project under Apache-2.0. No Tracee source code, eBPF/C programs,
signatures, generated API files, or native FreeBSD telemetry captures
were copied into this bootstrap.

This records engineering provenance, not a legal opinion. Uncertain
license questions and any future copied/adapted material require
file-level review and, where appropriate, human legal review before
distribution or commercialization.

## Architectural references reviewed

Tracee commit:
`2f9dc40c20b17c2ba27f6d92b25e62790bd48a62`.

- [Event schema](https://github.com/aquasecurity/tracee/blob/2f9dc40c20b17c2ba27f6d92b25e62790bd48a62/api/v1beta1/event.proto):
  inspected event IDs and timestamp/policy/workload/data/threat/source
  structure. BuoyBSD's schema is independent, not wire-compatible.
- [Output concepts](https://github.com/aquasecurity/tracee/blob/2f9dc40c20b17c2ba27f6d92b25e62790bd48a62/docs/docs/outputs/index.md)
  and [streams](https://github.com/aquasecurity/tracee/blob/2f9dc40c20b17c2ba27f6d92b25e62790bd48a62/docs/docs/outputs/streams.md):
  format/destination separation, bounded broadcast and independent
  diagnostics informed the design.
- [Tracee root license](https://github.com/aquasecurity/tracee/blob/2f9dc40c20b17c2ba27f6d92b25e62790bd48a62/LICENSE):
  Apache-2.0 at the root does not establish that every upstream
  subdirectory/file has identical terms.

Native facility references are linked in
[the feasibility document](freebsd-telemetry.md). They describe candidate
semantics; no online manual claim substitutes for a target-host test.

For future adaptations record: exact file/ref, applicable
LICENSE/NOTICE/directory terms, copied/modified/derived/independent
status, preserved copyright, meaningful changes, and semantic tests.

## Dependencies and generation

- `google.golang.org/protobuf` **v1.36.12**, BSD-3-Clause, runtime and
  `protoc-gen-go`; preserve its notice in
  [`licenses/protobuf.txt`](../licenses/protobuf.txt).
- `go.yaml.in/yaml/v3` **v3.0.5**, Apache-2.0 and MIT file-specific
  terms; preserve the complete dual-license notice in
  [`licenses/yaml.txt`](../licenses/yaml.txt).
- `honnef.co/go/tools` **v0.8.2**, MIT, development-only static analysis.
  Its tooling dependencies are pinned in `go.mod`/`go.sum`, not linked
  into BuoyBSD executables.
- Go **1.27.2** is the selected toolchain. Distributed Go binaries also
  include Go standard-library/runtime code under Go's BSD-style license;
  preserve [`licenses/go.txt`](../licenses/go.txt) in binary bundles.
- `protoc` **36.2** is an external development tool, not shipped with
  the sensor. Obtain official archives and verify their SHA-256.

Run `make generate` (or `go run ./tools/dev generate`) from the
repository root. The helper checks the protoc version, builds the Go
generator from the pinned module, and uses source-relative paths.
`make check-generated` regenerates into a temporary directory and
compares bytes without modifying tracked output.

`go run ./tools/dev notices` refreshes runtime dependency/Go license
snapshots from the verified module cache and selected GOROOT.
`make license` checks those snapshots against the selected dependencies,
the root license/NOTICE, and SPDX headers on authored Go/proto sources.
Dependency updates must refresh and review these notices.

Binary distributions must include LICENSE, NOTICE, and the `licenses`
directory. This bootstrap does not publish release archives.
