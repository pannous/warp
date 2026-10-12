# Delete-list cleanup

# Visualizer child-process exit cleanup

## Scope

- Reproduce the silent non-zero visualizer child exit with a focused regression before changing the helper.
- Keep environment-only WebGPU skips distinct from failed child processes, and make every failure report status, stdout, and stderr.
- Reuse one result-checking helper for both visualizer samples instead of adding per-test branches.

## Verification

- Run only the focused sound-spectrum/visualizer tests through `tests/queue.sh` before and after the change.
- Preserve the existing frame-count and sound-output assertions while proving a non-zero child cannot be silently skipped.
- Record the child-process testing rule under `notes/` and hand the branch and filter to the Integrator for the full gate.

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
