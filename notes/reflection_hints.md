# Reflection and standard-library hints

Reflection can make a standard-library function reachable without calling it. For example, `function repeat` and `methods("hi")` both compile `repeat(t, n) := n times t` from `lib/text.warp`.

The module loader parses and lowers standard modules without hints, and the final hint boundary also rejects diagnostics whose source is an embedded `lib/<module>.warp` file. This keeps implementation-style advice inside the library while preserving hints from the user's program compiled in the same run.

`tests/modules/test_std_reflection_hints.rs` pins both sides of that boundary: reflected standard functions are quiet, while an indexing hint in the user's source is still captured.
