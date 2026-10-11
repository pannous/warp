# Interactive console

The console recompiles each entered line together with earlier state-changing lines. Assignments, definitions,
compound assignments, increments and declarations therefore remain available to later input, while plain expressions
and output commands are not replayed.

This is source replay, not a long-lived WebAssembly instance. An assignment whose right side reads the clock, a file or
another changing external value runs again when later input is evaluated. A persistent run-time environment remains the
general solution for preserving those effects exactly once.

`probes/console_variables.py <warp binary>` drives the real readline console through a pseudo-terminal and checks the
basic `a=7`, then `a` contract without opening a window.
