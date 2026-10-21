# Dependency notices

These snapshots are generated from pinned module/GOROOT license files
by `go run ./tools/dev notices` and checked by `make license`:

- `protobuf.txt`: google.golang.org/protobuf, BSD-3-Clause.
- `yaml.txt`: go.yaml.in/yaml/v3, Apache-2.0 and file-specific MIT.
- `go.txt`: selected Go standard library/runtime, BSD-style.

Do not edit these notices by hand or remove upstream copyright text.
Ship this directory with LICENSE and NOTICE in any binary distribution.
Development-only Staticcheck and its dependencies are not linked into
BuoyBSD binaries. See [provenance](../docs/provenance.md).
