# JSON-to-Node cleanup

- Pin the existing foreign-call conversion behavior with a focused test before changing production code.
- Reuse one public-within-crate JSON conversion for foreign calls and headless event details.
- Preserve object order, array grouping and separators, integer/float handling, booleans, nulls, and the codepoint sentinel.
- Run the focused JSON and headless tests, formatting, clippy, and the wasm32 check.

# Delete-list cleanup

- Reproduce deletion of a list variable with a new focused probe test.
- Reuse the existing mutation lowering and list helpers rather than adding a special runtime path.
- Make deletion mutate the list variable consistently with list methods and preserve other delete forms.
- Run the focused test before and after, then clippy, wasm32 checking, and cargo fix.

# REPL variable persistence cleanup

- Keep the console's entered state in one small abstraction instead of spreading history handling through the readline loop.
- Re-evaluate only state-changing lines before the current expression, so ordinary queries and prints are not replayed.
- Add a headless console probe that exercises assignment followed by a variable read through the real CLI.
- Record the replay-based persistence boundary in the console notes, then run the focused CLI test, clippy, and wasm32 check.

# Fork-join scheduler cleanup

## Scope

- Replace native task-per-thread spawning with one fixed worker pool per program run.
- Keep task ids, copied values, task controls, result ordering, nested tasks, and error delivery unchanged.
- Bound concurrent worker threads to twice the machine's available parallelism while allowing queued tasks to start as workers become free. The second worker per core keeps blocking tasks from idling the CPU.

## Verification

- Establish the existing task-test baseline before implementation.
- Add a focused regression proving that more submitted tasks than workers complete through the bounded pool.
- Run the targeted task filters, clippy, and the wasm32 check through the project-prescribed commands.

## Cleanup

- Reuse `TaskTable`; do not add a second scheduler abstraction or a new dependency.
- Remove the per-task `JoinHandle` model once the pool owns worker handles.
- Keep the worktree-only Cargo version suffix uncommitted.

# wasm-split repair plan

- Reproduce the route-module failure with the existing focused test.
- Prefer the supported multi-split interface from the installed Binaryen toolchain.
- Preserve the primary-module ABI while collecting each named route module.
- Re-run the focused test, formatting, clippy, and the wasm32 check.

# Codex cron lifecycle cleanup

## Scope

- Keep scheduled runs as ordinary named app-server threads that are discoverable in the desktop app.
- Treat every terminal app-server state, connection loss, wrapper signal, and timeout as an explicit lifecycle outcome.
- Always close the app-server proxy and shell pipeline, atomically write a non-empty `.last.md`, and emit one accurate end notice.
- Preserve merge-request extraction and the existing `codex exec` fallback without concealing failures.

## Verification

- Capture the current live run and scheduler evidence without terminating or modifying it.
- Add focused real-process lifecycle probes under `probes/` for completed, interrupted, refused, disconnected, and timed-out outcomes where safely reproducible.
- Run the established targeted script checks before and after the change, then perform one tiny real Codex app-server probe if the remaining usage budget permits.

## Cleanup

- Reuse one lifecycle result representation and one final-artifact writer rather than adding shell-only special cases.
- Keep all failure details loud in the artifact, worker log, and Warp Boss end notice.
- Keep the worktree-only Cargo version suffix uncommitted.

# Runtime and parser source-position cleanup

## Scope

- Reproduce one parser error and one runtime error whose diagnostics omit the originating line and column.
- Preserve source metadata through the existing parse, lowering, emission, and runtime-error paths instead of introducing a parallel diagnostic representation.
- Report missing metadata loudly at the ownership boundary rather than silently inventing a position.

## Verification

- Establish focused parser and runtime diagnostic baselines through `tests/queue.sh` before implementation.
- Add focused regression coverage in the existing test hierarchy without changing established tests.
- Re-run only the affected queued filters and hand the full-suite gate to the Integrator.

## Cleanup

- Reuse the existing `Meta`, diagnostic, and runtime-error helpers.
- Remove any temporary probes or debug output once the focused tests pass.
- Keep the worktree-only Cargo version suffix uncommitted.

## Done (2026-10-12)

- Parser: `diagnostic::first_located_error` in the pipeline. Runtime: the `trap_position` global. Guide in
  notes/error_positions.md.
- Upgraded two exact-error pins (welcoming ask/slices) to include the new position.

# Quoted list-text cleanup

- Reproduce the self-hosted serializer's dynamically built quoted text inside a list before changing production code.
- Reuse the existing `text_quoted` runtime path for literal and computed texts instead of adding another serializer.
- Keep bare texts unquoted while ensuring container text reads back with quotes and backslashes escaped.
- Run the focused text test before and after, then formatting, clippy, and the wasm32 check.

# Standard-library documentation association cleanup

- Pin the reported markup and sound entries with a focused regression before changing the indexer.
- Keep the documented contract that a word uses the comment line directly above its definition.
- Reuse the existing single pending-comment slot; do not add module-specific exceptions or rewrite library comments.
- Run the focused documentation tests, formatting, clippy, and the wasm32 check.

# Repeated text-construction instruction cleanup

- Preserve the exact emitted instruction order while extracting the repeated pointer/length `new_text` sequences.
- Reuse helpers on `WasmEmitter`; keep callers responsible for computing any nonstandard pointer or length expression.
- Extract the repeated closure entry-field load into one helper without changing closure layout or call behavior.
- Establish and rerun focused text and closure test baselines, then run formatting, clippy, and the wasm32 check.

# Park natural phrases

- Remove the dedicated `phrase_calls`, `list_phrases`, and `word_slices` lowering passes and their parser marker.
- Keep the general call, method, filtering, sorting, and slice forms; rewrite repository samples to those forms.
- Mark only tests whose contract is the parked syntax as ignored with the card's reimplementation note; preserve tests
  for general preposition operators such as list/text `add … to …`.
- Remove stale documentation of the parked forms and record the retained explicit equivalents.
- Establish and rerun the focused phrase, list-phrase, and word-slice filters, then run formatting, clippy, and the
  wasm32 library check.
# Single-statement curly calls

## Done (2026-10-15)

- Library words now use the shared spaced-call predicate for a one-statement curly block and preserve the block wrapper.
- `if yes {sum [1 2]}` and `if yes {sum 1+1 2+2}` have focused regression coverage.

# Ignored-parameter function cleanup

- Reproduce a `def` function whose sole `_` parameter is discarded before changing production code.
- Keep `_` as an argument-consuming wildcard without treating it as a named local.
- Reuse the normal function-definition path so the rule also covers multiple ignored parameters and other bodies.
- Run the focused definition-form test before and after, then formatting, clippy, and the wasm32 check.

# Standard-library reflection hint cleanup

- Reproduce the leaked style hint through both `function repeat` and `methods("hi")` with a focused regression test.
- Find the reflection path that analyzes standard-library definitions outside the existing quiet module loader.
- Apply the standard-library-wide rule there, without hiding hints from the user's program.
- Run the focused test before and after, then formatting, clippy, and the wasm32 check.
- Record the diagnostic boundary in `notes/`, commit only the task files, merge current `origin/main`, push, and verify Playground CI.

# Parenthesized tuple-item cleanup

- Reproduce a comma tuple whose first item is a parenthesized `try … else …` expression with a focused control test.
- Keep the outer comma as the tuple separator; the inner parentheses group only the first value.
- Reuse the general bracket/list parser and avoid a `try`-specific rewrite.
- Run the focused parser/control tests, formatting, clippy, and the wasm32 check.

# Text builtin argument errors

- Reproduce `byte_at(3, 0)` in a new focused text test before changing production code.
- Route every non-literal `byte_at` text operand through the existing text-argument conversion so wrong kinds become the standard `not a text` error instead of a raw `ref.cast` trap.
- Preserve the inline byte-loop fast path, character-to-text conversion, error propagation, and bounds checking.
- Run the focused test before and after, then formatting, clippy, and the wasm32 check.
