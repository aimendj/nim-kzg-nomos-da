.PHONY: help setup build clean test test-rust

help:
	@echo "Available targets:"
	@echo "  setup            - Add, initialize, or update logos-blockchain submodule"
	@echo "  build-rust       - Build the Rust nomos-da library"
	@echo "  build            - Build the Nim wrapper"
	@echo "  clean            - Clean build artifacts"
	@echo "  test-rust        - Run Rust tests"
	@echo "  test             - Run Nim tests"

setup:
	@if [ -d "logos-blockchain" ]; then \
		echo "Submodule logos-blockchain exists. Updating..."; \
		git submodule update --remote logos-blockchain; \
	elif [ -f ".gitmodules" ]; then \
		echo "Initializing git submodule..."; \
		git submodule update --init --recursive; \
	else \
		echo "Adding logos-blockchain as git submodule..."; \
		git submodule add https://github.com/logos-blockchain/logos-blockchain.git logos-blockchain; \
	fi

build-rust:
	@echo "Building Rust FFI wrapper..."
	@if [ ! -d "logos-blockchain" ]; then \
		echo "Error: logos-blockchain submodule not found. Run 'make setup' first."; \
		exit 1; \
	fi
	@if [ ! -d "ffi-wrapper" ]; then \
		echo "Error: ffi-wrapper directory not found."; \
		exit 1; \
	fi
	cd ffi-wrapper && cargo build --release

build-nim:
	@echo "Building Nim wrapper..."
	nim c --path:src src/kzg_nomos_da.nim

clean:
	@echo "Cleaning build artifacts..."
	rm -rf nimcache
	rm -rf ffi-wrapper/target

test-rust:
	@echo "Running Rust tests..."
	@if [ ! -d "ffi-wrapper" ]; then \
		echo "Error: ffi-wrapper directory not found."; \
		exit 1; \
	fi
	cd ffi-wrapper && cargo test

test-nim:
	@echo "Running Nim tests..."
	nim c --path:src -r tests/test_encoder.nim
	nim c --path:src -r tests/test_verifier.nim
