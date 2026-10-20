# Diagnostics in warp (card pure-diagnostics, decision diagnostics-files)

The warp compiler's errors, warnings and hints are facts plus message files, no enums and no Rust bridge:

- A check (lib/compiler/kind_changes.warp, …) finds a fact: a node such as `kind_change(x, "Int", "List")`, and calls
  `diagnose(fact)` (lib/compiler/diagnostics.warp).
- `diagnose` looks the fact up in lib/compiler/errors.data, then warnings.data, then hints.data. Each row is
  `pattern TAB message`, read as plain warp data (parser.warp; `#` comments). The first row whose pattern matches wins:
  a `$name` part takes any node, keys and groups match part by part, `data(p)` quotes a pattern.
- The message fills `{name}` with the bound part and `{a name}` with it after its article (`an Int`).
- A fact no row matches is an error naming it, so a missing message is loud.
- The pure passes stage (passes.warp `checked`) answers `[view, diagnosed]`. src/compiler/self_hosted.rs
  `reported` makes an error the program's value, in both pure modes, and says warnings and hints through
  errors::diagnostic::report, which also fails them under `use strict`.

Ported so far: P45 kind changes (twin of src/analyzer/checks/kind_changes.rs). Not yet carried over:
- source positions: pure nodes don't carry any yet;
- walk.warp's `said` reports (number_words hints);
- whole-program source patterns (`data($x / 0)` matched against every node);
- the ~17 other tests of card pure-wrong-values.

Pitfalls met while writing it (each carded):
- An empty map `{}` is ø, so `m.get(k)` fails on it: use `k in m`.
- A loop variable named `each` breaks `each in m`.
- In the compiler's programs `symbol("List")` is node.warp's List case.
