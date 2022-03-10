# QOI

An implementation of the [QOI image format](https://qoiformat.org/) in [Rust](https://www.rust-lang.org/). Try it out at [qoi.pages.dev](https://qoi.pages.dev/).

## Directories / Crates

- [qoi](./qoi) - core QOI implementation, provides a decoder and encoder for the QOI image format
- [qoi_wasm](./qoi_wasm) - wrapper around the `qoi` crate that supports compiling to [WebAssembly](https://webassembly.org/)
- [site](./site) - experimental web page for decoding and encoding images to and from the QOI image format
- [site_util](./site_util) - internal crate that provides utility commands for building and developing [site](./site)

## Requirements

- [Rust](https://www.rust-lang.org/) >= `1.58`
- [Cargo](https://doc.rust-lang.org/cargo/) >= `1.58`
- The `wasm32-unknown-unknown` target needs to be installed if compiling the `qoi_wasm` crate

## Documentation

```sh
cargo doc --no-deps --workspace
```
