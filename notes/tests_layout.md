# tests/ layout

## Rules
- ONE test crate: `tests/main.rs` declares `mod common;` and one `mod <folder>;` per topic folder; each
  `tests/<folder>/mod.rs` declares that folder's files. A new test file goes into the folder of its topic and gets a
  `mod` line in that folder's mod.rs.
- File names keep their `test_` stem, so filters like `tests/queue.sh -- test_lists::` work (cargo filters are
  substrings). Full test names include the folder: `lists::test_lists::test_index`.
- No `probe_*.rs` in tests/: a probe's cases that matter become tests in the topic file, the rest is dropped.
- Only probes condense: a probe line already asserted by a test is dropped, the rest moves into the topic file.
  Assertions in regular test files stay where they are, even when duplicated. The test count drops only by probe
  tests that were duplicates; each one is named in its commit.
- `#[cfg(feature = "native")]` stays on the `mod` line of a native-only file (in the folder's mod.rs).
- Network: no third-party hosts like httpbin.org, and no mocks. A test of a request goes to a server we run:
  `common::serve(status, body)` (a fixed answer) or `common::echo_url()` (warp's own server answering POST /post with
  {data, headers}); in the browser the test page's /__stub__ (test_in_browser.py) answers both. Where a real public
  host is the point, it is pannous.com, and the test uses `is_online!` / `online_eval`: retried once, logged as
  `RETRY`, only when no answer came (connection, DNS, timeout), never on a wrong answer.
- Data files: tests/fixtures/, tests/warp/ (paths relative to the crate root).

## Folders
parser, node, numbers (incl. units, floats, laws), operators, types, lists, text, control (conditions, loops, match,
try, blocks, signals, tasks), functions, scope (globals, use scopes), modules, ffi (host, WASI, libm), wasm (emitter,
reader, names, optimizer, WIT), web, welcoming (footguns, got-it warnings, other languages' forms), programs
(algorithms, samples, kitchen sink, sound), sweeps (regression sweeps: test_todo's passing cases move to their topic
file), common (shared helpers).

## Child-process tests
A test running the warp binary (common::warp_command) reports a failed run whole: how it ended (`run.status`: an exit
code, or the signal that killed it under load), its stdout and stderr, as tests/programs/test_sound_spectrum.rs
run_report does. An environment skip (no WebGPU adapter) stays separate from a failure.

## Probes into tests, by judgment
A probe becomes test assertions only when it checks decided behaviour with a stable value; exploratory, visual,
sound, timing and undecided ones stay. Tests are outcome tests only (AGENTS.md "Outcome tests only").
Kept by class: markup with warnings, shaders and painting, sound, stored tables, timing, server/CLI, open questions,
design examples and spec inventories, probes/algo/ (bisection snippets; samples/<algo>.warp are the tests),
generators/, laws/ (Lean), tools and research (.py/.sh/.mjs/.md/.lean). Executable or user-touched probes (shebang,
+x, a "(user)" commit) are never deleted.
