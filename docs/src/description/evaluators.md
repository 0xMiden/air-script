# Evaluators
Evaluators are sets of constraints logically grouped together. The primary purpose of evaluators is to increase modularity and readability of AirScript code.

## Defining evaluators
An evaluator consists of a declaration which specifies evaluator metadata and a body which contains descriptions of integrity constraints.

Evaluator declaration starts with the `ev` keyword, followed by the name of the evaluator, parameter declarations, and a code block surrounded by braces. For example:
```
ev foo([a, b, c]) {}
```
Evaluator name must:
- Be a string consisting of alpha-numeric characters and underscores.
- Start with a letter.
- Be unique among the evaluators declared in and imported by a module.

Evaluator parameters define an evaluator's view into the execution trace. Specifically, they define the set of columns in the main trace segment the evaluator can access. For example, the evaluator declared above can access 3 columns of the main trace segment (which can be referenced as `a`, `b`, and `c`).

An evaluator body must contain at least one integrity constraint. For example:
```
ev foo([a, b]) {
    enf a' = a + b
}
```
In general, an evaluator body may contain any set of expressions allowed in the [integrity constraints](./constraints.md#integrity-constraints-integrity_constraints) section subject to the following caveats:

- Evaluators can access only the trace columns defined via its parameters.
- Evaluators can access only constants and periodic columns defined in the same module.

Evaluators can be declared anywhere in a module, but usually are declared towards the end of the module.

## Using evaluators
An evaluator defined in a module or [imported](./organization.md#importing-evaluators) from a different module can be invoked via the `enf` keyword. For example (public inputs and boundary constraints omitted for brevity):
```
trace_columns {
    main: [a, b],
}

integrity_constraints {
    enf foo([a, b]);
}

ev foo([x, y]) {
    enf x' = x + y;
}
```
In the above example, evaluator `foo` is invoked using trace columns `a` and `b`, but notice that within the evaluator we refer to these columns by different names (specifically, `x` and `y` respectively). The above example is equivalent to:
```
trace_columns {
    main: [a, b],
}

integrity_constraints {
    enf a' = a + b;
}
```
That is, we can think of evaluators as being *inlined* at their call sites.

Evaluators can be invoked multiple times. For example:
```
trace_columns {
    main: [a, b, c],
}

integrity_constraints {
    enf foo([a, b]);
    enf foo([c, a]);
}

ev foo([x, y]) {
    enf x' = x + y;
}
```
This is equivalent to:
```
trace_columns {
    main: [a, b, c],
}

integrity_constraints {
    enf a' = a + b;
    enf c' = c + a;
}
```
Evaluators can also invoke other evaluators. For example:
```
trace_columns {
    main: [a, b],
}

integrity_constraints {
    enf foo([a, b]);
}

ev foo([x, y]) {
    enf x' = x + y;
    enf bar([y, x]);
}

ev bar([x, y]) {
    enf x' = x * y;
}
```
The above is equivalent to:

```
trace_columns {
    main: [a, b],
}

integrity_constraints {
    enf a' = a + b;
    enf b' = b * a;
}
```

### Using in conditional constraints
Evaluators can also be used in [conditional constraints](./convenience.md#conditional-evaluators). The combination of evaluator and selector syntax is especially powerful as it enables describing complex constraints in a simple and modular way.
