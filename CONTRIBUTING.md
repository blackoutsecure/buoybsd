# Contributing to BuoyBSD

BuoyBSD is a replay-stage prototype. Read
[architecture](docs/architecture.md),
[security requirements](docs/security-model.md), and
[completion gates](docs/limitations.md) first.

## Tools

- Go 1.27.2; the module records the minimum/toolchain.
- Make (FreeBSD base Make or GNU Make) for the public interface.
- protoc 36.2 only for protobuf generation/consistency checks.
- No C/cgo for ordinary builds. Race testing requires a supported Go
  race platform and C compiler; CI runs it on Linux.
- The pinned module supplies `protoc-gen-go` and Staticcheck.

Install tools as an ordinary user. Verify official archive checksums.
Do not silently elevate, modify a shared host, or install global
dependencies merely to make validation pass.

## Commands

Run at the repository root. Direct equivalents also work on Windows:

| Make command | Direct Go equivalent |
| --- | --- |
| `make build` | `go run ./tools/dev build` |
| `make test` | `go test ./...` |
| `make test-unit` | `go test ./internal/... ./pkg/...` |
| `make test-integration` | `go test -tags=integration ./tests/integration/...` |
| `make test-race` | `go test -race ./...` |
| `make fmt` | `go run ./tools/dev fmt` |
| `make lint` | `go run ./tools/dev checkfmt`, `go vet ./...`, `go tool staticcheck ./...`, `go run ./tools/dev license` |
| `make generate` | `go run ./tools/dev generate` |
| `make check-generated` | `go run ./tools/dev check-generated` |
| `make license` | `go run ./tools/dev license` |
| `make build-freebsd` | `go run ./tools/dev build-freebsd` |
| `make clean` | `go run ./tools/dev clean` |

`make clean` removes only repository-local `bin` and `.tools` outputs.
Generated protobuf is committed. Do not hand-edit generated code.
After changing dependencies, run `go mod tidy`, refresh notices with
`go run ./tools/dev notices`, and review provenance.

The repository has no native FreeBSD runtime test yet. Replay
integration tests use a separate `integration` build tag and no
privileges. Future native tests need `freebsd` plus an explicit native
test tag/opt-in and must explain privileges before execution.

## Validation expectations

Use the smallest tests covering your change, then the relevant
cross-cutting checks above. CI runs formatting, vet, Staticcheck, unit
tests, Linux race tests, replay integration, protobuf consistency,
license checks, and **compilation only** for FreeBSD amd64/arm64.
CI checkouts do not persist repository credentials: these validation
jobs run contributed code and never need to push changes.

Tests should cover missing optional fields, malformed/oversized input,
unknown/duplicate keys, invalid UTF-8, real zero credentials, policy
semantics, detection source references, escaped tables, output errors,
webhook retries/redirects/timeouts, cancellation, and queue saturation.
Compare protobuf/JSON values rather than relying on JSON key order.

The bootstrap Windows host could not execute race-instrumented tests:
Go 1.27.2 with a portable Zig 0.17.0 C compiler linked successfully,
but ThreadSanitizer failed to allocate shadow memory (Windows error 87)
before application tests ran. Local race validation is therefore
**unverified**, not passed. Run the Linux CI race job on a published
change; do not disable instrumentation or alter host security settings
to turn this environment failure green.

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
