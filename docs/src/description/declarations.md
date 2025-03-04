# Type declaration sections

## Constants (`const`)

Constants can be optionally declared with the const keyword at the top of an AirScript module just below the declaration of the module name. They can be scalars, vectors or matrices. Constant names must contain only uppercase letters.

Each constant is defined by an identifier and a value in the following format:

```
const FOO = 123;
const BAR = [1, 2, 3];
const BAZ = [[1, 2, 3], [4, 5, 6]];
```

In the above example, `FOO` is a constant of type scalar with value `123`, BAR is a constant of type vector with value `[1, 2, 3]`, and BAZ is a constant of type matrix with value `[[1, 2, 3], [4, 5, 6]]`.

## Execution trace (`trace_columns`)

A `trace_columns` section contains declarations for `main` trace columns or `aux` (auxiliary) trace columns.

The `main` declarations define the shape of the main execution trace and define identifiers which can be used to refer to each of the columns or a group of columns in that trace. The columns can also be referred using the built-in variable `$main` and the index of the column in the trace.

**A `trace_columns` section with a `main` declaration is required for an AIR defined in AirScript to be valid.**

The following is a valid `trace_columns` source section:

```
trace_columns {
    main: [a, b, c[3], d],
}
```

In the above example, the main execution trace for the AIR has 6 columns with 4 column bindings, where the identifiers `a`, `b`, and `d` are each bound to a single column and `c` refers to a group of 3 columns. Single columns can be referenced using their identifiers (e.g. `a`, `b` and `d`) and columns in a group (e.g. `c`) can be referenced using the identifier `c` and the index of the column within the group `c` (`c[0]`, `c[1]` and `c[2]`).

## Public inputs (`public_inputs`)

A `public_inputs` section contains declarations for public inputs. Currently, each public input must be provided as a vector of a fixed size, but there is no limit to how many of them can be declared within the `public_inputs` section.

**Public inputs are required.** There must be at least one pubic input declared.

Each public input is described by an identifier and an array length (`n`) in the following format:

```
identifier: [n]
```

The following is an example of a valid `public_inputs` source section:

```
public_inputs {
    program_hash: [4],
    stack_inputs: [16],
    stack_outputs: [16],
}
```

In the above example, the public input `program_hash` is an array of length `4`. `stack_inputs` and `stack_outputs` are both arrays of length `16`.

Public inputs can be referenced by [boundary constraints](./constraints.md#boundary_constraints) by using the identifier and an index. For example, the 3rd element of the `program_hash` declared above would be referenced as `program_hash[2]`.

## Periodic Columns (`periodic_columns`)

A `periodic_columns` section contains declarations for periodic columns used in the description and evaluation of integrity constraints. Each periodic column declares an array of periodic values which can then be referenced by the declared identifier.

There is no limit to how many of them can be declared within the `periodic_columns` section.

**Periodic columns are optional.** It is equally valid to define an empty `periodic_columns` section or to omit the `periodic_columns` section declaration entirely.

Each periodic column is described by an identifier and an array of integers in the following format. These integers are the periodic values.

```
identifier: [i, j, k, n],
```

The length of each of the array must be a power of two which is greater than or equal to `2`.

The following is an example of a valid `periodic_columns` source section:

```
periodic_columns {
    k0: [0, 0, 0, 1],
    k1: [1, 1, 1, 1, 1, 1, 1, 0],
}
```

In the above example, `k0` declares a periodic column with a cycle of length `4`, and `k1` declares a periodic column with a cycle of length `8`.

Periodic columns can be referenced by [integrity constraints](./constraints.md#integrity_constraints) by using the column's identifier.

When constraints are evaluated, these periodic values always refer to the value of the column in the current row. For example, when evaluating an integrity constraint such as `enf k0 * a = 0`, `k0` would be evaluated as `0` in rows `0`, `1`, `2` of the trace and as `1` in row `3`, and then the cycle would repeat. Attempting to refer to the "next" row of a periodic column, such as by `k0'`, is invalid and will cause a `ParseError`.

## Buses (`buses`)

A `buses` section contains declarations for buses used in the description and evaluation of integrity constraints.

The following is an example of a valid `buses` source section:

```
buses {
    multiset p,
    logup q,
}
```

In the above example, we declare two buses: `p` of type `multiset`, and `q` of type `logup`. They respectively correspond to a multiset-based bus and a LogUp-based bus, that expand to different constraints. More information on bus types can be found in the [buses](./buses.md) section.
