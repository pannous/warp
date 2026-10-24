# ffi-python-adapter: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: #3089 general Python adapter: call any Python… ([ffi-python-adapter](https://github.com/pannous/warp/issues/3089))
- Branch: `ffi-python-adapter` at `71bb36d34`, 11 ahead / 436 behind origin/main `b8d2c4e99`; merges cleanly: no (conflicts: lib/compiler/c_imports.warp)
- Kind: warp only
- Owner: grok-warp-python (parked 2026-10-23; Supervisor: low priority): that owner continues it when resumed (the planner reassigns it otherwise).
- State: Call Python from warp through libpython (math.sqrt, str methods, json.dumps, exceptions); conflicts in lib/compiler c_imports.warp.

## Done
tests/host/test_python.sh, notes/python_adapter.md (measured after merging pure-c-imports).

## Missing
Resolve the c_imports.warp conflict, rerun test_python.

## How to test
Build warp-host from the merged tip, then tests/host/test_ffi.sh, test_ffi_sqlite.sh, test_ffi_z.sh, test_ffi_libc.sh (whichever the branch has) and the warp-only gate.

## Known failures
Unknown after a main merge.

## Next step
After pure-c-imports lands: merge main, rerun test_python, Ready.
