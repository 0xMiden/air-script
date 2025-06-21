#!/bin/sh

# Expects 
# 1. to be run from the root of the repository
# 2. for ~/.cargo/credentials.toml to contain your crates.io token (see
#   https://doc.rust-lang.org/cargo/reference/publishing.html)

cargo publish -p air-pass
cargo publish -p air-parser
cargo publish -p air-derive-ir
cargo publish -p air-mir
cargo publish -p air-ir
cargo publish -p air-codegen-ace
cargo publish -p air-codegen-winter
cargo publish -p air-script
