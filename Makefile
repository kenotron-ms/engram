.PHONY: install uninstall build test clean

## Install engram from source into ~/.cargo/bin (added to PATH by rustup)
install:
	cargo install --path crates/engram-cli --locked

## Remove the installed binary
uninstall:
	cargo uninstall engram

## Development build (fast, unoptimized)
build:
	cargo build

## Release build (optimized)
release:
	cargo build --release

## Run all tests
test:
	cargo test

## Remove build artifacts
clean:
	cargo clean
