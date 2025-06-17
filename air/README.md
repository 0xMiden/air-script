# Intermediate Representation (IR)

This crate contains the intermediate representation for AirScript, `AirIR`.

The purpose of the `AirIR` is to provide a simple and accurate representation of an AIR that allows for optimization and translation to constraint evaluator code in a variety of target languages.

## Generating the AirIR

Generate an `AirIR` from either an AirScript AST (the output of the AirScript parser) or a MIR (the Middle Intermediate Representation for AirScript).

Example usage:

```Rust
// parse the source string to a Result containing the AST or an Error
let ast = parse(source.as_str()).expect("Parsing failed");

// Create the compilation pipeline needed to translate the AST to AIR
let pipeline_with_mir = air_parser::transforms::ConstantPropagation::new(&diagnostics)
  .chain(mir::passes::AstToMir::new(&diagnostics))
  .chain(mir::passes::Inlining::new(&diagnostics))
  .chain(mir::passes::Unrolling::new(&diagnostics))
  .chain(air_ir::passes::MirToAir::new(&diagnostics))
  .chain(air_ir::passes::BusOpExpand::new(&diagnostics));

let pipeline_without_mir = air_parser::transforms::ConstantPropagation::new(&diagnostics)
  .chain(air_parser::transforms::Inlining::new(&diagnostics))
  .chain(air_ir::passes::AstToAir::new(&diagnostics));
  
// process the AST to get a Result containing the AIR or a CompileError
let air_from_ast = pipeline_without_mir.run(ast)
let air_from_mir = pipeline_with_mir.run(ast)
```

## AirIR

Although generation of an `AirIR` uses a symbol table while processing the source AST, the internal representation only consists of the following:

- **Name** of the AIR definition represented by the `AirIR`.
- **Segment Widths**, represented by a vector that contains the width of each trace segment (currently `main` and `auxiliary`).
- **Constants**, represented by a vector that maps an identifier to a constant value.
- **Public inputs**, represented by a vector that maps an identifier to a size for each public input that was declared. (Currently, public inputs can only be declared as fixed-size arrays.)
- **Periodic columns**, represented by an ordered vector that contains each periodic column's repeating pattern (as a vector).
- **Constraints**, represented by the combination of:
  - a directed acyclic graph (DAG) without duplicate nodes.
  - a vector of `ConstraintRoot` for each trace segment (e.g. main or auxiliary), where `ConstraintRoot` contains the node index in the graph where each of the constraint starts and the constraint domain which specifies the row(s) accessed by each of the constraints.
  - contains both boundary and integrity constraints.
