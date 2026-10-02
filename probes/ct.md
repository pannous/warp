# Cloud task brief (2026-09-29, decisions by the user)

You are one cloud session working on ONE task below (named in your prompt). A supervisor session reviews and merges.

## Rules
- Read CLAUDE.md first (project overview; `is!` round trip). Language decisions are in notes/open_decisions.md ("Decided").
- Branch `claude/<task-id>` from origin/main; commit and push after EVERY step (sessions can die mid-task, unpushed work is lost).
- Building: first fetch the vendored crates (notes/cloud_offline_build.md), then `cargo test --offline` works in the sandbox.
  Otherwise try `cargo build --offline`. If crates are unavailable in the sandbox, push anyway: GitHub CI ("Rust CI") runs
  `cargo test` on `claude/*` branches — check it with `gh run list --branch claude/<task-id>` / `gh run view` if gh works,
  and fix red runs. Never push code you have not either compiled or seen CI build.
- Tests: red test first (new file tests/test_<topic>.rs), then the fix. NEVER modify or delete existing tests, except
  where your task explicitly allows it. Removing an `#[ignore]` from a test that now passes unedited is allowed.
- Code: general rules, no per-case hacks; no duplication (extract helpers); constants at the top of the file; meaningful
  names; no trivial comments. Errors are values (`Node::Error`), never panics or silent wrong results.
- Commits: conventional (`fix:`, `feature(minor):`, `refactor:`, `test:`), NO Claude co-author or `Claude-Session:` lines.
- Don't merge into main yourself. Final answer: branch name, commit list, what changed, CI result, anything left open.

## Tasks

### J — juxtaposition and units (decisions #3, #5)
Unspaced number next to a symbol or `(` multiplies: `x=3;2x` → 6, `2π`, `3(4)` → 12, `⅓x`. Julia grouping: `1/2x` = `1/(2x)`,
`3x²` = `3·x²` (postfix binds to the symbol). Ordinal suffixes (`1st 2nd 3rd 4th`) are not products. `2e` = `2*e`, `2i` = `2*i`
(only if `i`/`e` resolve; otherwise the normal undefined-variable error). Data mode (`parse_data`) never multiplies.
Survey with details: the A10 report in notes/todo_sweep_task.md and wiki/number.md, wiki/unit.md, wiki/ambiguity.md.
Then units: `1 m + 1km` → `1001 m`, `3km+10m` → `3010 m` (unit values as identifiers, so `3km` = `3*km`); a SPACED
`2 km` multiplies only when `km` is a known unit (else it stays the list `[2 km]`). `1950 ± 50` (a value with tolerance),
`1900 - 2000 AD` (a range with a unit). Un-ignore test_implicit_multiplication, test_units, test_hyphen_units if they pass
unedited.

### E — undefined calls are errors, print with hidden IO (decisions #7, #8)
Implement notes/unresolved_call_survey.md: `name(` without space, in emitted code, that no user function / import /
builtin / type constructor resolves → `Error('undefined function: name')`; data stays data. `P(1)` for a declared type
`P` constructs it. Add builtins `min`, `max`. Add `print x` / `print(x)`: prints and returns x; eval grants the needed IO
capability implicitly (hidden from the user: no capability error for print). Un-ignore test_print / test_print_function
if they pass unedited.

### S — size, types, data as scope (decisions #4, #6, #9)
`size` is a synonym of count (element count; text: character count). Bytes only via `byte count`, `number of bytes`,
`#bytes in list`. Update wiki/Footguns.md's recorded "size counts bytes" decision accordingly.
`type([1 2 3])` → `list of int`; plural type words denote lists (`numbers`, `ints`, `x:numbers`).
Data as scope: in `a-b:2 c-d:4 a-b`, a symbol resolves to its key's value (→ 2), unless `a` and `b` are also variables,
then give a warning (kebab name vs subtraction ambiguity). Un-ignore test_array_size, test_array_operations,
test_array_type_generics, test_hypen_versus_minus if they pass unedited.

### M — `use <file>` modules (decision #12)
`use sin` loads `sin.wasp`/`sin.warp` from the current dir, then samples/, then the library path, and imports its
definitions (functions, globals, types) once; `use math` stays the builtin. Missing file → `Error('module not found: sin')`.
test_sinus_wasp_import should pass unedited.

### F — exact reals into float, shifts, no panics (decisions #11, #14, #1)
`float x = π` (and `x:float = √2`, `= 1/3`) is ALLOWED: a declared float target accepts an exact real with the precision
loss (the one explicit exception to "no silent loss"). Shift operators `<<` `>>` on exact Ints (today `2 << 1` silently
gives 0): implement them for Int, float operands → the float-in-exact-context error. Then a panic sweep: every remaining
`panic!`/`unwrap()`/`expect()`/`todo!()` reachable from user programs in src/wasm_emitter, src/analyzer.rs,
src/wasp_parser.rs becomes an error value (keep genuine internal-invariant asserts); list what you kept and why.

### C — test defects and cleanup (user: "don't care, cleanup!")
Permitted test edits: test_sin → compare with a tolerance; test_named_data_sections → remove the `exit(0)` line;
test_comments2 → expect the Rust model (`(y=0)` is a Key, length 0) or assert the 2-statement shape; test_paint_wasm →
assign `w`, write `(x - c)`; the commented `"a".s() + 2` lines in test_string.rs may be deleted. Remove the WIT emitter
paragraph from CLAUDE.md and AGENTS.md (src/wit_emitter.rs does not exist). Delete tests/test_footgun_application.rs and
tests/test_footgun_list_index_bounds.rs (duplicates of tests/probe_footguns.rs). Report which tests now pass.
