# Licensing and provenance

## Original implementation

BuoyBSD's Rust code, protobuf schema, configuration, documentation, and
synthetic replay envelopes are independently implemented for this
project under Apache-2.0. The Rust implementation ports the project's
own former Go replay prototype; no Go production code or fallback is
retained. No Tracee source code, eBPF/C programs,
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

Rust **1.99.1** is pinned in `rust-toolchain.toml`. `Cargo.lock` records
exact crate versions and registry checksums; ordinary builds use
`--locked`. Domain code uses the Rust standard library and narrow
Serde/thiserror support, not Tokio, Reqwest, protobuf, OS acquisition,
HTTP servers, or database implementations.

Principal adapters use Prost **0.14.4** (Apache-2.0), pbjson **0.9.0**
(MIT), Serde/serde_json (MIT OR Apache-2.0), serde_yaml_ng (MIT OR
Apache-2.0), Clap (MIT OR Apache-2.0), Tokio/tokio-util (MIT), Reqwest
(MIT OR Apache-2.0), native-tls (MIT OR Apache-2.0), SHA-2 (MIT OR
Apache-2.0), and Tracing (MIT). Consult
[`licenses/runtime-index.json`](../licenses/runtime-index.json) for
the exact, target-inclusive runtime dependency closure, source
locations, SPDX expressions, and SHA-256 of each preserved notice.
The generated index, not this overview, is the version inventory.
pbjson 0.9 omits its workspace-root LICENSE from the crate archive.
`licenses/upstream-sources.json` preserves the full MIT text verified
against its packaged VCS commit and the exact upstream file digest;
the notice checker verifies both before including it. This is not an
assumed generic MIT substitution or an unpinned network download.
Packaged AUTHORS attribution files are retained as well; r-efi's
AUTHORS file contains its actual triple-license grant and copyright.

Prost-build **0.14.4** and pbjson-build **0.9.0** are feature-gated
developer dependencies, not sensor runtime dependencies. `protoc`
**36.2** is an external development executable, not shipped with the
sensor. Obtain official archives and verify their SHA-256.

Run `make generate` (or
`cargo run --locked --features tooling --bin buoy-dev -- generate`).
The helper checks the protoc version and generates the two Rust
bindings beside the unchanged `event.proto`. It deterministically
removes pbjson 0.9's redundant `&FIELDS` formatting borrow so strict
Rust 1.99 Clippy remains enabled; it does not suppress that check.
`make check-generated` regenerates under `.tools/protobuf-check` and
compares bytes without replacing tracked output.

The timestamp adapter implements protobuf's canonical UTC `Z` and
0/3/6/9 fractional-digit representation rather than delegating to a
generic date serializer. Numbers, scalar presence, oneof tags, raw
bytes, enum names, and 64-bit JSON strings retain the existing
contract. The schema's historical `go_package` option remains inert
metadata for existing schema consumers, not a Go build dependency.

`make notices` refreshes source-license snapshots from Cargo's
checksum-verified cache and the selected Rust toolchain's
`COPYRIGHT-library.html`/license texts (the `rust-docs` component).
`make license` checks exact bytes, stale snapshots, root attribution,
and SPDX headers on authored Rust/protobuf sources. The dependency
closure follows normal dependencies across target conditions;
development/build-only tools are not represented as runtime code.
Review file-specific and target-system licensing before distributing
binaries, including OpenSSL or GNU runtime components as applicable.

Binary distributions must include LICENSE, NOTICE, and the `licenses`
directory, plus any additional target-system notices required by the
actual linked libraries. This prototype publishes no release archives.

## Migration evidence

The schema and original synthetic replay input are unchanged.
Meaningful replay, normalization, policy, detector, output, webhook,
configuration, pipeline, CLI, and integration regressions were ported.
The added subprocess JSON golden records the former executable's
synthetic output; see [fixture provenance](../tests/fixtures/README.md).
No saved output or cross-platform compilation upgrades a Tracee
mapping's status or certifies native FreeBSD collection.
