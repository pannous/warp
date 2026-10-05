# Cloud pilot report: claude/shared-kind-constants

- Start (UTC): 2026-10-03T10:46:49Z
- End (UTC): 2026-10-03T11:09:59Z
- First build + baseline suite (cold, incl. vendor extraction): 729 s total; test run itself 543.5 s, so cold build ≈ 185 s
- Incremental test build after refactor: 29 s
- Full suite duration after refactor (incl. rebuild of changed crate): 544 s
- Baseline totals: 1442 passed, 7 failed, 81 ignored
- Final totals: 1442 passed, 7 failed, 81 ignored (identical failing set)
- Files changed: src/type_kinds.rs, src/web.rs, src/node.rs, src/wasm_reader.rs, src/run/wasmtime_runner.rs, src/wasm_emitter/{mod,equality,library_ops,list_ops,text_builtins,text_unicode,exact}.rs

## Problems
The 7 baseline failures are environmental (same before and after):
- test_law (2) and probe_footguns::test_proof_model_matches_unbounded_int: `lean` not installed in the sandbox.
- test_package_tools (2), test_uniscript::the_index_matches_the_readable_entities, test_package_pin::header_signatures_are_parsed_once: package sub-builds need crate `miniz_oxide`, which is missing from the vendor branch / offline (`no matching package named miniz_oxide`), so vendor/ may need a refresh for packages/uniscript.
- No network, toolchain or timeout problems. The foreground command limit (600 s) was hit by the first suite run, which moved to background; use run_in_background for the suite.
