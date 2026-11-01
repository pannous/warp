# Runtime documentation

The current compiler lowers `core/runtime.warp` and `core/runtime/extended.warp` together for `runtime.wasm`. The core-to-extended boundary uses `bridge.warp` hooks and is checked by `scripts/self-check.sh`. `wiki/runtime.md` documents the split and examples checked on the native compiler. The `tests-leave` card needs changes to existing tests or to the core runner, whose working directory is the test file's folder; both were outside the worker's authorization.
