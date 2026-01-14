# nim-kzg-nomos-da

Nim bindings for nomos-da from [logos-blockchain](https://github.com/logos-blockchain/logos-blockchain).

Supports encoding. Verification and reconstruction are work in progress.

```bash
# Get the submodule
make setup

# Build the Rust FFI lib
make build-rust

# Build the Nim wrapper
nimble build
```

## Project layout

```
nim-kzg-nomos-da/
├── logos-blockchain/     # submodule
├── ffi-wrapper/          # Rust FFI crate
├── src/                  # Nim code
├── API.md                # API docs
└── Makefile
```

## Building

Just use the Makefile:

```bash
make setup      # init submodule
make build-rust # build Rust lib
make build      # build everything
make clean      # clean up
```

Or build manually:

```bash
cd ffi-wrapper && cargo build --release
nimble build
```

The Rust build outputs `libnomos_da_ffi.so` (or `.dylib`/`.dll`) in `ffi-wrapper/target/release/`.

## Usage

```nim
import nomos_da

let result = nomos_da_init()
if result == Success:
  nomos_da_cleanup()
```
