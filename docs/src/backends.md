# Backends
AirScript currently comes bundled with two backends:

- [Winterfell backend](https://github.com/0xPolygonMiden/air-script/tree/main/codegen/winterfell) which outputs `Air` trait implementation for the [Winterfell prover](https://github.com/facebook/winterfell) (Rust).

These backends can be used programmatically as crates. They can also be used via AirScript CLI by specifying `--target` flag.

For example, the following will output Winterfell `Air` trait implementation for AIR constraints described in `example.air` file:
```
./target/release/airc transpile examples/example.air --target winterfell
```
In both cases we assumed that the CLI has been compiled as described [here](./introduction.md#cli).
