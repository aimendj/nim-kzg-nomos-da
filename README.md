# nim-kzg-nomos-da

Nim bindings for nomos-da from [logos-blockchain](https://github.com/logos-blockchain/logos-blockchain).

Supports encoding, verification, serialization and reconstruction.

```bash
# Get the submodule
make setup

# Build the Rust FFI library (static)
make build-rust

# Build the Nim wrapper
make build-nim

# Run tests
make test-rust  # Rust tests
make test-nim   # Nim tests
```

## Building

### Using Makefile (Recommended)

```bash
make setup      # Initialize/update submodule
make build-rust # Build Rust static library
make build-nim  # Build Nim wrapper
make test-rust  # Run Rust tests
make test-nim   # Run Nim tests
make clean      # Clean build artifacts
```

The Rust build outputs `libnomos_da_ffi.a` (static library) in `ffi-wrapper/target/release/`.

## Linking

The library is statically linked. The `nim.cfg` file automatically configures the linker:

```nim
passl:"-L./ffi-wrapper/target/release"
passl:"-lnomos_da_ffi"
```

## Testing

```bash
# Run Rust FFI tests
make test-rust

# Run Nim wrapper tests
make test-nim
```
