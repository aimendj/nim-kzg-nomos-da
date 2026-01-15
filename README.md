# nim-kzg-nomos-da

Nim bindings for nomos-da from [logos-blockchain](https://github.com/logos-blockchain/logos-blockchain).

Supports encoding. Verification and reconstruction are work in progress.

```bash
# Get the submodule
make setup

# Build the Rust FFI library (static)
make build-rust

# Build the Nim wrapper
make build

# Run tests
make test-rust  # Rust tests
make test       # Nim tests
```

## Building

### Using Makefile (Recommended)

```bash
make setup      # Initialize/update submodule
make build-rust # Build Rust static library
make build      # Build everything (Rust + Nim)
make test-rust  # Run Rust tests
make test       # Run Nim tests
make clean      # Clean build artifacts
```

The Rust build outputs `libnomos_da_ffi.a` (static library) in `ffi-wrapper/target/release/`.

## Linking

The library is statically linked. The `nim.cfg` file automatically configures the linker:

```nim
passl:"-L$projectPath/ffi-wrapper/target/release"
passl:"-lnomos_da_ffi"
```

## Testing

```bash
# Run Rust FFI tests
make test-rust

# Run Nim wrapper tests
make test
```
