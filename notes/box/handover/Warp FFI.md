# Warp FFI handover (2026-10-23 14:32 Berlin). All trees clean and pushed; no runs left going.
- pure-c-imports 3d9802b7c (main d9490fc59 merged): fix = warp-host takes compiler beside it before lib/../data. Last board state seen: Ready at 836b8e9a4; Ready at 3d9802b7c not confirmed. Last measured (836b8e9a4): ffi 21, host 68, lists 2, launcher 22.
- ffi-sqlite3 cdddd6d51: sqlite3 in pure Warp, 48 C fns, checks now positive; test_ffi_sqlite 20/20 measured before the positive rework. Not Ready (stacked).
- ffi-z 06b1a8852: zlib + bytes results (buffer/written), c_write/c_load/c_store, 1 MiB OK; test_ffi_z 22/22 before positive rework. Not Ready (stacked on ffi-sqlite3).
- ffi-libc 02a9f22f4: libc as foreign lines, c_errno, hand-linked shims only for rust-lane modules; test_ffi_libc 10/11 last run (c_errno, likely stale compiler). Not Ready (stacked on ffi-z).
- pass-owners-table 36c13bceb: card pass-order-owners steps 1+2+4 (passes.warp order + pass_owners/owned_refusal, test_pass_owners.sh). Not built/tested, not Ready.
Next:
1. Build pure-c-imports (rebuild pure via /workspace/cow/sol-warp-latest + warp-host build, box-slot.sh), run 4 suites, `todo ready pure-c-imports pure-c-imports` + card note (root cause: stale data/compiler-self.wasm).
2. Re-verify ffi-sqlite3/ffi-z/ffi-libc after the positive-test rework; ready each once its base merges.
3. pass-order-owners step 3 (c_imports into definitions, drop text pre-pass + c_without), approved by Supervisor.
4. Later: raylib/SDL2 (Mac), wasmtime (generic struct layout), AOT .cwasm lacks warp.c_calls letters.
Open: tests must be result-only (recheck my new test scripts for exit-code/error-wording pins); cards host-int-precision, pure-any-is, pure-param-data filed for compiler owners.
