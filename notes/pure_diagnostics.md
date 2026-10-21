# Diagnostics in warp (card pure-diagnostics, decision diagnostics-files)

The warp compiler's errors, warnings and hints are facts plus message files, no enums and no Rust bridge:

- A check (lib/compiler/kind_changes.warp, …) finds a fact: a node such as `kind_change(x, "Int", "List")`, and calls
  `diagnose(fact)` (lib/compiler/diagnostics.warp).
- `diagnose` looks the fact up in lib/compiler/errors.data, then warnings.data, then hints.data. Each row is
  `pattern TAB message`, read as plain warp data (parser.warp; `#` comments). The first row whose pattern matches wins:
  a `$name` part takes any node, keys and groups match part by part, `data(p)` quotes a pattern.
- The message fills `{name}` with the bound part and `{a name}` with it after its article (`an Int`).
- After the message a row may give a fix (`TAB "global {name}"`, shown after `; fix: `) and then
  `TAB "topic@expression"`, what "got it" silences (Diagnostic::about): `"and-or@{condition} and {middle} or {otherwise}"`.
- A fact no row matches is an error naming it, so a missing message is loud.
- The pure passes stage (passes.warp `checked`) answers `[view, diagnosed]`, each diagnosed entry
  `[level, message, fix, topic@expression]`. src/compiler/self_hosted.rs `diagnosed` makes an error the program's
  value, in both pure modes; the warnings and hints are said through errors::diagnostic::report once the pure lane
  made the module (a later refusal leaves them to the Rust lane, which says its own), failing under `use strict`
  (in_source_mode) and skipping what was acknowledged with "got it".

Where the checks run (`checked`), as the Rust pipeline orders them:
- on parser.warp's nodes as written, attribute layers kept (source_checks.warp): an error the parser left in the tree
  (a negative index, sugar.warp subscript), `"3" + 3`, bare `sleep(1)`, dropped copies (`uppercase x;`) and errors,
  `y = x.upper!`, and the lints of lints.warp (`x and 0 or 2`, `7 % -2`, `0.1 as float == 0.1`, `a * b as T`);
- on the lowered program: kind changes (kind_changes.warp, P45), then main-level variables changed from a function
  (main_globals.warp: an error without `global n`, a local-or-global warning for a fresh `n = 5`). Only the
  program's own functions are checked: the prelude's and used files' definitions come in with `with_used`.

Not yet carried over:
- source positions: pure nodes don't carry any yet, so no `at line:column`, and no fix edits (Diagnostic.fixes,
  test_fixits applies them at the diagnostic's position) nor the fractional-index errors' `at 1:36`
  (card pure-lane-welcoming);
- `use folder` / `use package` / `use project` scopes (test_use_scopes, card pure-lane-use-2);
- walk.warp's `said` reports (number_words hints), the hidden-`it` and braced-`it` warnings, kebab ambiguities;
- whole-program source patterns (`data($x / 0)` matched against every node).

Pitfalls met while writing it (each carded):
- An empty map `{}` is ø, so `m.get(k)` fails on it: use `k in m`.
- A loop variable named `each` breaks `each in m`.
- In the compiler's programs `symbol("List")` is node.warp's List case.
- A local, parameter or loop variable named like a function of the compiler's files (`variable`, `definition`,
  `index`, `position`, `written`, `then`) is taken as that function: "not a function", "takes 1 argument"
  (card parameter-named-like-2).
- Two files defining the same top-level constant (`statement_separators`) make it a reassigned variable that
  functions can't read: define it once (walk.warp).
- `for x in v` over an any-typed local holding a list of keys fails "not an int"; a list-typed function's result
  iterates (card rust-lane-typed).
