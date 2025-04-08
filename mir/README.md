# Middle Intermediate Representation (MIR)

This crate contains the middle intermediate representation for AirScript, `MIR`.

The purpose of the `MIR` is to provide a representation of an AirScript program that allows for optimization and translation to `AirIR` containing the `AlgebraicGraph`

## Generating the MIR

Generate an `MIR` from an AirScript AST (the output of the AirScript parser) using the `new` method. The `new` method will return a new `MIR` or an `Error` of type `SemanticError` if it encounters any errors while processing the AST.

The `new` method will first iterate through the source sections that contain declarations to build a symbol table with constants, trace columns, public inputs, periodic columns and random values. It will return a `SemanticError` if it encounters a duplicate, incorrect, or missing declaration. Once the symbol table is built, the constraints and intermediate variables in the `boundary_constraints` and `integrity_constraints` sections of the AST are processed. Finally, `new` returns a Result containing the `AirIR` or a `SemanticError`.

Example usage:

```Rust
// parse the source string to a Result containing the AST or an Error
let ast = parse(source.as_str()).expect("Parsing failed");

// Create the compilation pipeline needed to translate the AST to MIR
let pipeline = air_parser::transforms::ConstantPropagation::new(&diagnostics)
  .chain(mir::passes::AstToMir::new(&diagnostics))

// process the AST to get a Result containing the MIR or a CompileError
let mir = pipeline.run(ast)
```

## AirIR

Although generation of an `AirIR` uses a symbol table while processing the source AST, the internal representation only consists of the following:

- **Name** of the AIR definition represented by the `AirIR`.
- **Segment Widths**, represented by a vector that contains the width of each trace segment (currently only `main`).
- **Constants**, represented by a vector that maps an identifier to a constant value.
- **Public inputs**, represented by a vector that maps an identifier to a size for each public input that was declared. (Currently, public inputs can only be declared as fixed-size arrays.)
- **Periodic columns**, represented by an ordered vector that contains each periodic column's repeating pattern (as a vector).
- **Constraints**, represented by the combination of:
  - a directed acyclic graph (DAG) without duplicate nodes.
  - a vector of `ConstraintRoot` for each trace segment (e.g. main), where `ConstraintRoot` contains the node index in the graph where each of the constraint starts and the constraint domain which specifies the row(s) accessed by each of the constraints.
  - contains both boundary and integrity constraints.
