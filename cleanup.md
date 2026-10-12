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
