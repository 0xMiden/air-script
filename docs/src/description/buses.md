# Buses

A bus is a construct which aims to simplify description of non-local constraints defined via multiset or LogUp checks.

## Bus types

- Multiset (`unit`): Multiset-based buses can represent constraints which specify values that must have been added or removed from a column, in no particular order.
[Citation needed]()

- LogUp (`mult`): LogUp-based buses are more complex than multiset buses, and can encode the multiplicity of an element: an element can be added or removed multiple times.
[Multivariate lookups based on logarithmic derivatives - Ulrich Haböck, Orbis Labs, Polygon Labs](https://eprint.iacr.org/2022/1530)

## Defining buses

See the [declaring buses](./declarations.md#buses) for more details.

```
buses {
    unit p,
    mult q,
}
```

## Bus boundary constraints

In the boundary constraints section, we can constrain the initial and final state of the bus. Currently, only constraining a bus to be empty (with the  `null` keyword) is supported.

```
boundary_constraints {
    enf p.first = null;
    enf p.last = null;
}
```

The above example states that the bus `p` should be empty at the beginning and end of the trace.

## Bus integrity constraints

In the integrity constraints section, we can add and remove elements (as tuples of felts) to and from a bus. In the following examples, `p` and `q` are respectivelly multiset and LogUp based buses.

```
integrity_constraints {
    p.add(a) when s1;
    p.rem(a, b) when 1 - s2;
}
```

Here, `s1` and `1 - s2` are binary selectors: the element is added or removed when the corresponding selector's value is 1.

The global resulting constraint on the column of the bus is the following: `$p ′ ⋅ ( ( α_0 + α_1 ⋅ a + α_2 ⋅ b ) ⋅ ( 1 − s2 ) + s2 ) = p ⋅ ( ( α_0 + α_1 ⋅ a ) ⋅ s1 + 1 − s1 ))$`, where `$α i$` corresponds to the i-th random value provided by the verifier.

```
integrity_constraints {
    q.rem(e, f, g) when s
    q.add(a, b, c) for d
}
```

Similarly to the previous example elements can be added or removed from `q` with binary selectors. However, as it is a LogUp-based bus, it is also possible to add and remove elements with a given scalar multiplicity with the `for` keyword (here, `d` does not have to be binary).

The global resulting constraint on the column of the bus is the following: `$q ′ + s ( α_0 + α_1 ⋅ e + α_2 ⋅ f + α_3 ⋅ g ) = q + d ( α_0 + α_1 ⋅ a + α_2 ⋅ b + α_3 ⋅ c )$`, where `$α i$` corresponds to the i-th random value provided by the verifier.
