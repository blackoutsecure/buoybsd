# BuoyBSD

[![Made by BlackoutSecure](https://img.shields.io/badge/Made_by-BlackoutSecure-000000)](https://github.com/blackoutsecure)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

**Deep runtime visibility for FreeBSD.**

BuoyBSD is an experimental runtime-security, observability, and
forensics sensor for FreeBSD.

It is being designed to collect security-relevant activity using
FreeBSD-native facilities, normalize it into a stable event model,
evaluate detections, and route events to structured outputs.

The project explores compatibility with Aqua Security Tracee's event
and output model where FreeBSD provides semantically equivalent data.

BuoyBSD is an independent open-source project. It is not affiliated
with, sponsored by, or endorsed by Aqua Security, the Tracee project,
or the FreeBSD Project.

## Status: replay prototype, not a live sensor

Implemented:

- Rust daemon/control-client entry points using Clap.
- A versioned, independently designed `buoybsd.v1beta1` protobuf API.
- Bounded, strict replay input for a process-execution vertical slice.
- Normalization with field presence, original record preservation,
  source attribution, and incomplete-data warnings.
- Exact/glob policies and an informational pure-Rust shell detector.
- Protobuf JSON to stdout/files, escaped tables, and bounded webhooks.
- Unit, replay, subprocess CLI, and concurrency regressions; configured
  FreeBSD amd64/arm64 compilation checks.

**No production auditpipe or DTrace collector exists yet.** Supplied
fixtures are synthetic replay-contract examples, not captured FreeBSD
telemetry. No event is certified Tracee-equivalent. Passing a cross-build
does not establish FreeBSD runtime support.

The GitHub repository remains `blackoutsecure/int-buoybsd`; the project
and installed commands are named BuoyBSD, `buoybsd`, and `buoyctl`.

## Quick start

Install Rust **1.99.1**, a platform linker, and the Unix OpenSSL
development libraries where applicable. Ordinary builds do not need
Go or protobuf tools. From the repository root:

```sh
cargo build --locked --bin buoybsd --bin buoyctl
cargo run --locked --bin buoybsd -- --config configs/buoybsd.example.yaml
cargo run --locked --bin buoybsd -- --replay tests/fixtures/process_exec.synthetic.ndjson --output table
cargo run --locked --bin buoyctl -- events list
cargo test --locked --all-targets --all-features
```

Event output goes to stdout; structured diagnostic logs and final
counters go to stderr. An input or delivery error produces a nonzero
exit status. No root privileges are needed for replay.

Use `make` (including FreeBSD's base `make`) as the public developer
interface:

```sh
make build
make test
make test-unit
make test-integration
make test-concurrency
make lint
make fmt
make generate
make check-generated
make notices
make license
make build-freebsd
make clean
```

Executables are written to `target/debug` (`.exe` on Windows).
`make generate` requires **protoc 36.2**; Prost and protobuf-JSON
generators are pinned in Cargo. See [CONTRIBUTING.md](CONTRIBUTING.md)
for direct commands, Windows GNU linking, TLS-test prerequisites, and
FreeBSD build prerequisites. No live FreeBSD runtime is certified.

## Configuration and safety

Start with [the example configuration](configs/buoybsd.example.yaml).
Unknown YAML keys, unsupported collectors, invalid selectors, excessive
buffers, and contradictory destinations are rejected before collection.
CLI options override only explicitly selected configuration values.

Replay is lossless under normal backpressure: its bounded queue blocks
rather than silently discarding records. A failed destination stops the
pipeline. Delivery to several outputs is not transactional.

Webhook delivery uses verified HTTPS, finite timeouts and retries, no
redirect following, and synchronous backpressure. Cleartext HTTP is
available only with explicit opt-in for loopback testing. Raw records
and command arguments may contain sensitive information; protect output
files and endpoints accordingly.

Do not deploy this prototype as a security boundary or run telemetry
experiments on a production host.

## Design and evidence

- [Architecture and scope](docs/architecture.md)
- [Event model and replay contract](docs/event-model.md)
- [FreeBSD feasibility and next experiment](docs/freebsd-telemetry.md)
- [Tracee compatibility matrix](docs/tracee-compatibility.md)
- [Limitations and completion gates](docs/limitations.md)
- [Security model](docs/security-model.md)
- [Provenance and dependency licenses](docs/provenance.md)
- [Contributing and exact validation commands](CONTRIBUTING.md)
- [Vulnerability reporting](SECURITY.md)

The first live-sensor POC is **not complete** until a supported FreeBSD
host produces a captured process-execution fixture, its semantic gaps
are reviewed, and opt-in native integration tests pass. The smallest
proposed experiment is documented, not automatically executed.

## License

Original BuoyBSD code is licensed under [Apache-2.0](LICENSE).
See [NOTICE](NOTICE) and [dependency notices](licenses/README.md).
Tracee is an architectural reference, not a copied implementation.

Copyright 2026 BlackoutSecure contributors.
