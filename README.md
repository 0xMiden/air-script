# AirScript

<a href="https://github.com/0xMiden/air-script/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
<img src="https://github.com/0xMiden/air-script/workflows/CI/badge.svg?branch=main">
<a href="https://crates.io/crates/air-script"><img src="https://img.shields.io/crates/v/air-script"></a>

A domain-specific language for expressing AIR constraints for STARKs, especially for STARK-based virtual machines like [Miden VM](https://github.com/maticnetwork/miden/).

An in-depth description of AirScript is available in the full AirScript [documentation](https://0xMiden.github.io/air-script/).

**WARNING**: This project is in an alpha stage. It has not been audited and may contain bugs and security flaws. This implementation is NOT ready for production use.

## Overview

AirScript is a domain-specific language for writing AIR constraints for the STARK proving system. The primary goal of AirScript is to enable writing and auditing AIR constraints without the need to learn a specific programming language (e.g., Rust). The secondary goal is to perform automated optimizations of constraints and to output constraint evaluator code in multiple target languages (e.g., Rust, Miden assembly, Solidity etc.).

## Project Structure

The project is organized into several crates as follows:
| Crate | Description |
| ---------------------- | ----------- |
| [Parser](parser) | Contains the parser for AirScript. The parser is used to parse the constraints written in AirScript into an AST. |
| [IR](ir) | Contains the IR for AirScript, `AirIR`. `AirIR` is initialized with an AirScript AST, which it converts to an internal representation that can be optimized and used to generate code in multiple target languages. |
| [Winterfell code generator](codegen/winterfell/) | Contains a code generator targeting the [Winterfell prover](https://github.com/novifinancial/winterfell) Rust library. The Winterfell code generator converts a provided AirScript `AirIR` into Rust code that represents the AIR as a new custom struct that implements Winterfell's `Air` trait. |
| [Miden assembly code generator](codegen/masm/) | Contains code generator targeting the [Miden VM](https://github.com/0xPolygonMiden/miden-vm). Miden assembly code generator converts `AirIR` into Miden assembly module contain constraint evaluation and related logic. |
| [AirScript](air-script) | Aggregates all components of the AirScript compiler into a single place and provides a CLI as an executable to transpile AIRs defined in AirScript to the specified target language. Also contains integration tests for AirScript. |

## Contributing to AirScript

AirScript is an open project and we welcome everyone to contribute! If you are interested in contributing to AirScript, please have a look at our [Contribution guidelines](https://github.com/0xMiden/air-script/blob/main/CONTRIBUTING.md). If you want to work on a specific issue, please add a comment on the GitHub issue indicating you are interested before submitting a PR. This will help avoid duplicated effort. If you have thoughts on how to improve AirScript, we'd love to know them. So, please don't hesitate to open issues.

## References

1. [Logos](https://github.com/maciejhirsz/logos/): Library for generating fast lexers in Rust.
1. [LALRPOP](https://github.com/lalrpop/lalrpop/): LR(1) Rust parser generator framework.
1. [Codegen](https://github.com/carllerche/codegen): Library for generating Rust code.
1. [mdBook](https://github.com/rust-lang/mdBook): Utility for creating online documentation books.

## License

This project is [MIT licensed](./LICENSE).
