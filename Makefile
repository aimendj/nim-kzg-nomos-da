.PHONY: help setup build clean test test-rust

help:
	@echo "Available targets:"
	@echo "  setup            - Initialize and update git submodules"
	@echo "  build-rust       - Build the Rust nomos-da library"
	@echo "  build            - Build the Nim wrapper"
	@echo "  clean            - Clean build artifacts"
	@echo "  test-rust        - Run Rust tests"
	@echo "  test             - Run Nim tests"

setup:
	@if [ -f ".gitmodules" ]; then \
		echo "Initializing git submodules..."; \
		git submodule update --init --recursive; \
		echo "Updating git submodules..."; \
		git submodule update --remote --recursive; \
	fi
	@if [ ! -d "logos-blockchain" ]; then \
		echo "Adding logos-blockchain as git submodule..."; \
		git submodule add https://github.com/logos-blockchain/logos-blockchain.git logos-blockchain; \
	fi
	@if [ ! -d "nim-bincode" ]; then \
		echo "Adding nim-bincode as git submodule..."; \
		git submodule add https://github.com/aimendj/nim-bincode.git nim-bincode; \
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
	@if [ -d "nim-bincode" ]; then \
		echo "Building nim-bincode Rust library..."; \
		cd nim-bincode && make build || cargo build --release; \
	fi

build-nim:
	@echo "Building Nim wrapper..."
	nim c --path:src src/kzg_nomos_da.nim

clean:
	@echo "Cleaning build artifacts..."
	rm -rf nimcache
	rm -rf ffi-wrapper/target
	@if [ -d "nim-bincode" ]; then \
		cd nim-bincode && make clean 2>/dev/null || true; \
		rm -rf nim-bincode/target; \
	fi

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
	nim c --path:src -r tests/test_share.nim
	nim c --path:src -r tests/test_reconstruction.nim