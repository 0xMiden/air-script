# Miden assembly code generator

This crate contains a code generator targeting the [Miden VM](https://github.com/0xPolygonMiden/miden-vm).

The purpose of this code generator is to convert a provided `AirIR` representation of an AIR into a custom Miden assembly module that contains constraint evaluation logic for this AIR. The generated code can be used with the recursive STARK proof verifier in Miden standard library.
