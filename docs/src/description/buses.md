# Buses

A bus is a construct which aims to simplify description of non-local constraints defined via multiset or LogUp checks.

## Bus types

- Multiset (`multiset`): Multiset-based buses can represent constraints which specify values that must have been inserted or removed from a column, in no particular order.
[Miden VM - Multiset Checks](https://0xpolygonmiden.github.io/miden-vm/design/lookups/multiset.html)
[Incremental Multiset Hash Functions and Their Application to Memory Integrity Checking - Clarke et al. MIT CSAIL (2018)](https://people.csail.mit.edu/devadas/pubs/mhashes.pdf)

- LogUp (`logup`): LogUp-based buses are more complex than multiset buses, and can encode the multiplicity of an element: an element can be inserted or removed multiple times.
[Miden VM - LogUp: multivariate lookups with logarithmic derivatives](https://0xpolygonmiden.github.io/miden-vm/design/lookups/logup.html)
[Multivariate lookups based on logarithmic derivatives - Ulrich Haböck, Orbis Labs, Polygon Labs](https://eprint.iacr.org/2022/1530)

## Defining buses

See the [declaring buses](./declarations.md#buses) for more details.

```
buses {
    multiset p,
    logup q,
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

In the integrity constraints section, we can insert and remove elements (as tuples of felts) into and from a bus. In the following examples, `p` and `q` are respectively multiset and LogUp based buses.

```
integrity_constraints {
    p.insert(a) when s1;
    p.remove(a, b) when 1 - s2;
}
```

Here, `s1` and `1 - s2` are binary selectors: the element is inserted or removed when the corresponding selector's value is 1.

The global resulting constraint on the column of the bus is the following, where $\alpha_i$ corresponds to the i-th random value provided by the verifier: $$p ′ \cdot ( ( \alpha_0 + \alpha_1 \cdot a + \alpha_2 \cdot b ) \cdot ( 1 − s2 ) + s2 ) = p \cdot ( ( \alpha_0 + \alpha_1 \cdot a ) \cdot s1 + 1 − s1 ))$$

```
integrity_constraints {
    q.remove(e, f, g) when s
    q.insert(a, b, c) for d
}
```

Similarly to the previous example elements can be inserted or removed from `q` with binary selectors. However, as it is a LogUp-based bus, it is also possible to add and remove elements with a given scalar multiplicity with the `for` keyword (here, `d` does not have to be binary).

The global resulting constraint on the column of the bus is the following, where $\alpha_i$ corresponds to the i-th random value provided by the verifier: $$q ′ + \frac{s}{ \alpha_0 + \alpha_1 \cdot e + \alpha_2 \cdot f + \alpha_3 \cdot g } = q + \frac{d}{ \alpha_0 + \alpha_1 \cdot a + \alpha_2 \cdot b + \alpha_3 \cdot c}$$

If we respectively note $ v_+ = \alpha_0 + \alpha_1 \cdot a + \alpha_2 \cdot b + \alpha_3 \cdot c $ and $ v_- = \alpha_0 + \alpha_1 \cdot e + \alpha_2 \cdot f + \alpha_3 \cdot g$ the tuples inserted into and removed from the bus, the equation can be rewritten: $$( q ′ - q ) \cdot v_+ \cdot v_- = s \cdot v_+  + d \cdot v_- $$
