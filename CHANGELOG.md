# Changelog

## 0.4.0 (2025-06-20)

### Language

- [BREAKING] Introduced "block" syntax for all declarations and statements (#350).
- Implemented support for pure functions (#351).
- [BREAKING] Introduced semicolons as statement terminators (#353).
- Allowed named constants in rage notation (#355).
- Added commas in match/case statements (#356).
- Implemented `bus` constructs (#371).
- Removed `aux` and `rand` values from the frontend (#375).
- Implemented variable length public input (for bus boundaries) (#376).
- Implemented nested list comprehension (#400).

### Codegen

- Introduced initial version of the ACE backend (#370, #380, #386).
- Updated Winterfell codegen to the latest version (#388).
- Removed obsolete MASM codegen backend (#389).

### Internal

- Eliminated exponentiations in the IR (#352).
- Updated compilation pipeline to use MIR (#359).
- Incremented MSRV to 1.87.

## 0.3.0 (2023-07-12)

- Added support for library modules.
- Added support for evaluator functions.
- Added support for constraint comprehension.
- Added support for conditional constraints.
- Refactored parser to make it more robust and provide better error reporting.
- Fixed grammar ambiguities.
- Added initial implementation of Miden assembly backend.

## 0.2.0 (2023-02-23)

- Added support for named constants (scalars, vectors, and matrices).
- Added support for intermediate variables in `boundary_constraints` and `integrity_constraints` sections (scalars, vectors, and matrices).
- Added support for binding identifier names to groups of trace columns (in addition to single columns, which were already supported).
- Added the `$main` and `$aux` built-ins for accessing columns in the execution trace by index.
- [BREAKING] Replaced the `$rand` built-in for accessing random values with an explicit `random_values` declaration section, enabling use of a custom identifier for the random values array.
- Added support for binding identifiers to specific random values or groups of random values.
- Made significant changes to the IR, including:
  - Moved the boundary constraints into the algebraic constraint graph.
  - Made the trace reference more general to support additional trace segments in the future.
  - Added analysis to differentiate between validity and transition constraints.
  - Added the `Sub` operation and removed the `Neg` operation.
- [FIX] Fixed a bug in the Winterfell codegen output for auxiliary transition constraints.
- Improved the Winterfell codegen by consolidating code generation for boundary and transition constraints and removing redundant parentheses.

## 0.1.0 (2022-11-10)

- Initial release of AirScript, including a minimal but complete implementation of the AirScript compiler for an initial basic set of language features.
