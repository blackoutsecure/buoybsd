CARGO ?= cargo

.PHONY: all build test test-unit test-integration test-concurrency lint fmt generate check-generated notices license build-freebsd clean

all: build

build:
	$(CARGO) build --locked --bin buoybsd --bin buoyctl

test:
	$(CARGO) test --locked --all-targets --all-features
	$(CARGO) test --locked --doc --all-features

test-unit:
	$(CARGO) test --locked --all-features --lib --test domain --test config --test replay --test output --test pipeline --test webhook

test-integration:
	$(CARGO) test --locked --test integration --test cli --test commands

test-concurrency:
	$(CARGO) test --locked --test pipeline --test replay --test webhook

lint:
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --locked --all-targets --all-features -- -D warnings
	$(CARGO) run --locked --features tooling --bin buoy-dev -- license

fmt:
	$(CARGO) fmt --all

generate:
	$(CARGO) run --locked --features tooling --bin buoy-dev -- generate

check-generated:
	$(CARGO) run --locked --features tooling --bin buoy-dev -- check-generated

notices:
	$(CARGO) run --locked --features tooling --bin buoy-dev -- notices

license:
	$(CARGO) run --locked --features tooling --bin buoy-dev -- license

build-freebsd:
	$(CARGO) build --locked --target x86_64-unknown-freebsd --bin buoybsd --bin buoyctl
	$(CARGO) build --locked --target aarch64-unknown-freebsd --bin buoybsd --bin buoyctl

clean:
	$(CARGO) run --locked --features tooling --bin buoy-dev -- clean
	$(CARGO) clean --target-dir target
