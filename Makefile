.PHONY: help setup build clean test

help:
	@echo "Available targets:"
	@echo "  setup            - Add, initialize, or update logos-blockchain submodule"
	@echo "  build-rust       - Build the Rust nomos-da library"
	@echo "  build            - Build the Nim wrapper"
	@echo "  clean            - Clean build artifacts"
	@echo "  test             - Run tests"

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

build: build-rust
	@echo "Building Nim wrapper..."
	nimble build

clean:
	@echo "Cleaning build artifacts..."
	rm -rf nimcache
	rm -rf ffi-wrapper/target
	rm -f src/nomos_da

test:
	@echo "Running tests..."
	nimble test
