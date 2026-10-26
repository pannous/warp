# test/ layout

## Rules
- Tests are warp files, outcome tests only (AGENTS.md "Outcome tests only"): `test/<topic>/test_*.pairs.warp` (code ⇥
  expected, one a line) and `test_*.warp` (`is!` lines), notes/is_tests.md. Run them with `warp test [folder…]`.
- A new test goes into the folder of its topic. No probe files in test/: a probe's cases that matter become tests in the
  topic file, the rest is dropped. Probes condense, never the other way; a probe line already asserted by a test is
  dropped. Assertions in regular test files stay where they are, even when duplicated.
- Network: no third-party hosts like httpbin.org, and no mocks. A test of a request goes to a server we run, or to
  pannous.com where a real public host is the point.
- Data files: test/fixtures/, test/warp/.

## Folders
parser, node, numbers (incl. units, floats, laws), operators, types, lists, text, control (conditions, loops, match,
try, blocks, signals, tasks), functions, scope (globals, use scopes), modules, ffi (host, WASI, libm), wasm (emitter,
reader, names, optimizer, WIT), web, welcoming (footguns, got-it warnings, other languages' forms), programs
(algorithms, samples, kitchen sink, sound), sweeps (regression sweeps), host and launcher (shell tests of the harness).

## Probes into tests, by judgment
A probe becomes test assertions only when it checks decided behaviour with a stable value; exploratory, visual,
sound, timing and undecided ones stay. Kept by class: markup with warnings, shaders and painting, sound, stored tables,
timing, server/CLI, open questions, design examples and spec inventories, probes/algo/ (bisection snippets;
samples/<algo>.warp are the tests), generators/, laws/ (Lean), tools and research (.py/.sh/.mjs/.md/.lean).
Executable or user-touched probes (shebang, +x, a "(user)" commit) are never deleted.
