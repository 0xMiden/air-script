# Example AIR in AirScript

This is an example AIR definition in AirScript that includes all existing AirScript syntax. It is intended to be syntactically demonstrative rather than meaningful.

```
def ExampleAir

trace_columns {
    main: [s, a, b, c],
}

public_inputs {
    stack_inputs: [16],
    stack_outputs: [16],
}

periodic_columns {
    k0: [1, 1, 1, 1, 1, 1, 1, 0],
}

buses {
    logup: q,
}

boundary_constraints {
    # define boundary constraints against the main trace at the first row of the trace.
    enf a.first = stack_inputs[0];
    enf b.first = stack_inputs[1];
    enf c.first = stack_inputs[2];

    # define boundary constraints against the main trace at the last row of the trace.
    enf a.last = stack_outputs[0];
    enf b.last = stack_outputs[1];
    enf c.last = stack_outputs[2];

    # set the bus q to be initially empty
    enf q.first = null;
}

integrity_constraints {
    # the selector must be binary.
    enf s^2 = s;

    # selector should stay the same for all rows of an 8-row cycle.
    enf k0 * (s' - s) = 0;

    # c = a + b when s = 0.
    enf (1 - s) * (c - a - b) = 0;

    # c = a * b when s = 1.
    enf s * (c - a * b) = 0;

    # insert p to the q bus when s = 1
    q.insert(p) when s;
}
```
