# test/ layout

## Rules
- Tests are warp files, outcome tests only (AGENTS.md "Outcome tests only"): `test/<topic>/test_*.pairs.warp` (code ⇥
  expected, one a line) and `test_*.warp` (`is!` lines), notes/is_tests.md. Run them with `warp test [folder…]`.
- A new test goes into the folder of its topic; every kept test lives in test/ (probes/ is gone since 2026-10-26, its
  files are in git history). Experiments go to the session scratchpad or data/<topic>/ and become test lines or are
  dropped. Assertions in regular test files stay where they are, even when duplicated.
- Network: no third-party hosts like httpbin.org, and no mocks. A test of a request goes to a server we run, or to
  pannous.com where a real public host is the point.
- Data files: test/fixtures/, test/warp/.

## Folders
parser, node, numbers (incl. units, floats, laws), operators, types, lists, text, control (conditions, loops, match,
try, blocks, signals, tasks), functions, scope (globals, use scopes), modules, ffi (host, WASI, libm), wasm (emitter,
reader, names, optimizer, WIT), web, welcoming (footguns, got-it warnings, other languages' forms), programs
(algorithms, samples, kitchen sink, sound), sweeps (regression sweeps), host and launcher (shell tests of the harness),
gate (the gate's tier 1 programs and their .expected output, scripts/gate.sh).
