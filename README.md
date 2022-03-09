# QOI

An implementation of the [QOI image format](https://qoiformat.org/) in [Rust](https://www.rust-lang.org/).

## Directories / Crates

- [site](./site) - an experimental web page that provides a GUI for decoding and encoding images to and from the QOI image format via `qoi_wasm`
- [site_util](./site_util) - internal crate that provides utility commands for building and developing [site](./site)
- [qoi](./qoi) - lib crate that contains the core QOI implementation, provides a decoder and encoder for the QOI image format
- [qoi_wasm](./qoi_wasm) - wrapper for the `qoi` crate that supports compiling to [WebAssembly](https://webassembly.org/)

## Requirements

- [Rust](https://www.rust-lang.org/) >= `1.58`
- [Cargo](https://doc.rust-lang.org/cargo/) >= `1.58`
- The `wasm32-unknown-unknown` target needs to be installed if compiling the `qoi_wasm` crate

## Documentation

```sh
cargo doc --no-deps --workspace
```
