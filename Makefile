GO ?= go

.PHONY: all build test test-unit test-integration test-race lint fmt generate check-generated license build-freebsd clean

all: build

build:
	$(GO) run ./tools/dev build

test:
	$(GO) test -timeout 120s ./...

test-unit:
	$(GO) test -timeout 120s ./internal/... ./pkg/...

test-integration:
	$(GO) test -timeout 120s -tags=integration ./tests/integration/...

test-race:
	$(GO) test -timeout 120s -race ./...

lint:
	$(GO) run ./tools/dev checkfmt
	$(GO) vet ./...
	$(GO) tool staticcheck ./...
	$(GO) run ./tools/dev license

fmt:
	$(GO) run ./tools/dev fmt

generate:
	$(GO) run ./tools/dev generate

check-generated:
	$(GO) run ./tools/dev check-generated

license:
	$(GO) run ./tools/dev license

build-freebsd:
	$(GO) run ./tools/dev build-freebsd

clean:
	$(GO) run ./tools/dev clean
