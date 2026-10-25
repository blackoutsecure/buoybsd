# Contributing to BuoyBSD

BuoyBSD is a replay-stage prototype. Read
[architecture](docs/architecture.md),
[security requirements](docs/security-model.md), and
[completion gates](docs/limitations.md) first.

## Tools

- Rust 1.99.1; `rust-toolchain.toml` and Cargo record the selection.
- Make (FreeBSD base Make or GNU Make) for the public interface.
- protoc 36.2 only for protobuf generation/consistency checks.
- A working platform linker. Unix builds need OpenSSL headers/libraries
  and `pkg-config` (`pkgconf` on FreeBSD); TLS uses the native system
  trust store. Windows uses SChannel rather than vendored OpenSSL.
- The OpenSSL CLI for TLS regression tests. Tests generate ephemeral
  self-signed identities under `target/test-data`, not committed keys.
  Set `BUOYBSD_TEST_OPENSSL` if it is not on PATH; Windows also discovers
  Git for Windows' bundled OpenSSL.
- Clippy and rustfmt for checks; `rust-docs` for reproducible Rust
  library license snapshots. The pinned Cargo graph supplies Prost
  0.14.4 and pbjson 0.9.0 for generation.

Install tools as an ordinary user. Verify official archive checksums.
Do not silently elevate, modify a shared host, or install global
dependencies merely to make validation pass.

## Commands

Run at the repository root. Direct equivalents also work on Windows:

| Make command | Direct Rust equivalent |
| --- | --- |
| `make build` | `cargo build --locked --bin buoybsd --bin buoyctl` |
| `make test` | `cargo test --locked --all-targets --all-features` then `cargo test --locked --doc --all-features` |
| `make test-unit` | `cargo test --locked --all-features --lib --test domain --test config --test replay --test output --test pipeline --test webhook` |
| `make test-integration` | `cargo test --locked --test integration --test cli --test commands` |
| `make test-concurrency` | `cargo test --locked --test pipeline --test replay --test webhook` |
| `make fmt` | `cargo fmt --all` |
| `make lint` | `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, then the license command below |
| `make generate` | `cargo run --locked --features tooling --bin buoy-dev -- generate` |
| `make check-generated` | `cargo run --locked --features tooling --bin buoy-dev -- check-generated` |
| `make notices` | `cargo run --locked --features tooling --bin buoy-dev -- notices` |
| `make license` | `cargo run --locked --features tooling --bin buoy-dev -- license` |
| `make build-freebsd` | `cargo build --locked --target x86_64-unknown-freebsd --bin buoybsd --bin buoyctl` then the same command with `--target aarch64-unknown-freebsd` |
| `make clean` | `cargo run --locked --features tooling --bin buoy-dev -- clean` then `cargo clean --target-dir target` |

`make clean` removes repository-local `bin`, `.tools`, and Cargo
`target` outputs, not an externally selected `CARGO_TARGET_DIR`.
The helper refuses non-directory/symlink cleanup roots.
Generated protobuf and JSON bindings are committed. Do not hand-edit
them. After deliberate dependency changes, update `Cargo.lock`, run
`cargo fetch --locked`, refresh `make notices`, and review provenance.
Generation checks use `.tools/protobuf-check`, never an OS temporary
directory. `PROTOC`/`PROTOC_INCLUDE` can select a repository-local
checksum-verified protoc installation.

The repository has no native collector/runtime test yet. Rust replay
integration tests run by default, without privileges. Future native
tests need FreeBSD `cfg` gating, an explicit separate opt-in, and an
explanation of privileges before execution. No such feature currently
enables native acquisition.

### Windows GNU linking

Add the user-local Cargo bin directory to the current process PATH.
Do not change the global toolchain or require administrator access.
Rust's GNU distribution may have `rust-lld.exe` but no complete GCC
installation. A tested process-only setup is:

```powershell
$rustBin = Join-Path $env:USERPROFILE '.rustup\toolchains\1.99.1-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin'
$env:PATH = "$env:USERPROFILE\.cargo\bin;$rustBin;" + $env:PATH
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $rustBin 'rust-lld.exe'
$env:RUSTFLAGS = '-Clink-self-contained=yes'
```

Raw Windows DLL imports also need a working `dlltool` **and assembler**.
The bundled `dlltool` alone can fail with `CreateProcess` because `as`
is missing. The migration used ignored repository-local MSYS2 binutils
2.47-4, verified against SHA-256
`e36c52a7fe1d104c2efd97dc74162386eabf8323f6aeb9b83aeb574b291a7da0`,
from [the official package metadata](https://packages.msys2.org/packages/mingw-w64-x86_64-binutils).
Its `mingw64/bin`, Git for Windows' `mingw64/bin` (dependency DLLs), and
the Rust directory above were prepended only to the validation process
PATH. No compiler or user-global configuration was installed.
A trivial Rust smoke build does not establish dependency-link support.

### FreeBSD builds

`make build-freebsd` requires both Rust targets **and matching target
linkers, C runtime/sysroots, and OpenSSL headers/libraries**. It fails
if those prerequisites are absent; it is not silently replaced with
`cargo check`. CI instead installs checksum-verified Rust 1.99.1 into
repository-local directories in FreeBSD 15.1 amd64 and arm64 VMs and
performs full target-native executable builds using base OpenSSL.
Emulated arm64 compilation has a longer, finite CI job budget.
Those jobs are compilation gates, not native sensor experiments or
evidence that this unpublished migration has already passed FreeBSD CI.

## Validation expectations

Use the smallest tests covering your change, then the relevant
cross-cutting checks above. CI runs formatting, strict Clippy, unit/TLS
tests, replay/subprocess integration, bounded concurrency regressions,
doctests, protobuf consistency, license checks, and **compilation only**
for FreeBSD amd64/arm64.
CI checkouts do not persist repository credentials: these validation
jobs run contributed code and never need to push changes.

Tests should cover missing optional fields, malformed/oversized input,
unknown/duplicate keys, invalid UTF-8, real zero credentials, policy
semantics, detection source references, escaped tables, output errors,
webhook retries/redirects/timeouts, cancellation, and queue saturation.
Compare protobuf/JSON values rather than relying on JSON key order.

Stable Rust concurrency tests are not Go's race detector or a
ThreadSanitizer run. No sanitizer result is claimed. Saturation,
cancellation, retries, resource ownership, and partial-broadcast
accounting have executable regressions. Former nil-interface cases
are unrepresentable by Rust's required references/trait objects.
The Go-only environment-override helper is replaced by explicit Cargo
`--target` arguments, not an emulation of `GOOS`/`CGO_ENABLED`.

Never describe a skipped test as passed. Record platform/tool
limitations and exact command/results in a PR.

## Fixtures and telemetry experiments

Existing fixtures are explicitly synthetic replay examples, not
invented BSM/DTrace captures. Follow
[the experiment plan](docs/freebsd-telemetry.md) before adding a native
parser. Captures must be real, sanitized, accompanied by provenance,
and free of secrets/environments/customer or unrelated host data.

Never run destructive tests or change production audit, modules, pf,
permissions, or privilege configuration.

## Pull requests

Use a feature branch and the repository PR template. Keep one complete
behavioral slice per change, with tests and directly related docs.
Describe the problem, source and privileges, security implications,
exact validation results, compatibility effect, and known gaps.
Use imperative scoped conventional commit subjects when practical.
Do not invent issue references.
