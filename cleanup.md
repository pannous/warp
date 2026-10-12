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
