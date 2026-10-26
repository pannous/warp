# Wasmer for warp: feasibility (2026-10-21, warp-wasmer)

Asked by the user: "evaluate the feasibility of Wasmer, in general". The investigation was stopped early by the user;
items 1 and 2 are probed, 3–4 partly sourced, the rest marked open.

## Verdict: stay wasmtime-only; Wasmer is blocked on WASM GC
Wasmer 7.5.0 (latest, 2026-10-01) cannot load any module warp emits, on any backend. GC types (struct, array, anyref)
are refused before compiling. Revisit only when Wasmer's `Value`/type layer gains GC references. Until then the
decision wasm-standalone's two-step route (warp.wasm writes file.wasm, then `wasmtime file.wasm`) stays the answer.
A WASIX one-command compile-and-run stays a Later idea.

## 1. Proposal support, probed (probes/wasmer/features.sh, outputs in scratch/wasmer-probes)
Homebrew's wasmer 7.5.0 has only Cranelift. The official release tarball
([v7.5.0](https://github.com/wasmerio/wasmer/releases/tag/v7.5.0), `wasmer --version -v`: "runtimes: Singlepass,
Cranelift, LLVM, V8") has all four; `WASMER=<it> probes/wasmer/features.sh` runs every backend.

| proposal | Cranelift | LLVM | Singlepass | V8 | wasmtime 49 |
|---|---|---|---|---|---|
| exceptions (try_table) | yes | yes | no | yes | yes |
| multi-memory | yes | yes | yes | yes | yes |
| tail calls | no | yes | no | yes | yes |
| typed function refs (call_ref) | no | no | no | yes | yes |
| i31ref | no | no | no | yes | yes |
| GC struct / array | **no** | **no** | **no** | **no** ("GC is not implemented yet") | yes |

- warp's `samples/hello/hello.warp` → `warp compile --wasm` (10 KB) fails on all four backends ("array indexed
  types not supported without the gc feature"; V8: "GC is not implemented yet").
- `/Users/me/dev/bin/warp.wasm` on V8: "only numeric types are supported in function signatures: Unsupported
  ref type: anyref". It also imports 6 custom `host` functions (read, run, fetch, fetch_within, std_io, warn) plus
  WASI fd_write, so it would need those host words built in (card standalone) on any runtime.
- `--enable-all` makes every backend refuse even an MVP module ("No backends support the required features").
- The reason is not V8 itself, which runs GC in Chrome. Wasmer's shared type layer has no GC references:
  [`wasmer::Value`](https://docs.rs/wasmer/7.2.1/wasmer/enum.Value.html) is I32, I64, F32, F64, V128, ExternRef,
  FuncRef, ExceptionRef. The 7.5.0 binary contains the same list plus the "GC is not implemented yet" string.
- Upstream status: [#357 "gc support"](https://github.com/wasmerio/wasmer/issues/357) was closed after a maintainer
  said GC "should be supported if you are using Wasmer with the V8 backend". The probe above contradicts that for
  7.5.0. [#6387](https://github.com/wasmerio/wasmer/issues/6387) (GC) was closed 2026-04 with no comment;
  [#6280](https://github.com/wasmerio/wasmer/issues/6280) (typed function references) is still open. No GC roadmap.

## 2. WASIX: compile and run in one command
- [proc_spawn](https://wasix.org/docs/api-reference/wasix/proc_spawn) / proc_exec take a command name. In wasmer-wasix
  `lib/wasix/src/bin_factory/mod.rs` (v7.5.0), `load_executable_from_filesystem` loads that name from the virtual
  filesystem as a webc package or as a raw `.wasm` (`Executable::Wasm` → `spawn_exec_wasm`). So warp.wasm could
  in principle write file.wasm and then proc_spawn it.
- Moot while no backend runs GC. Even then warp would have to import `wasix_32v1` proc_spawn/proc_join (Wasmer-only
  ABI, not WASI 0.1/0.2) and keep a two-step fallback for wasmtime.
- No in-process instantiation exists in WASI or WASIX. A module cannot compile and link another module itself; only
  the host can.

## 3. Embedding the wasmer crate (from docs, not tried)
- Same shape as wasmtime: Store, Module, Instance, `imports!`, Function::new_typed(_with_env). Backends are cargo
  features (sys with cranelift, llvm or singlepass; v8; wamr; wasmi; js).
- Not usable for warp: `Value` has no anyref/structref/arrayref, so src/run/wasmtime_runner.rs and src/gc_traits/
  (GC object introspection: reading struct fields, `Node::from_gc_object`) have no counterpart. Host functions can't
  take or return warp's Node refs.
- No component model / WIT support in the wasmer crate (none on [docs.rs](https://docs.rs/wasmer/latest/wasmer/));
  wasmtime has it. docs.rs failed to build 7.5.0, so 7.2.1 is the last documented release.

## 4. Other factors (partly checked)
- License MIT (GitHub API); ~21 k stars; pushed daily (2026-10-20); releases about every 2–4 weeks (7.2.1 Jul → 7.5.0 Oct).
- The release tarball for darwin-arm64 is 286 MB (it includes the V8, LLVM and headless builds).
- [wasmer-js / @wasmer/sdk](https://github.com/wasmerio/wasmer-js) runs WASIX packages in the browser via
  wasm-bindgen. warp's browser build already runs its modules in JavaScript with the browser's own GC, so it gains
  nothing there.
- Open, not measured: startup and run speed against wasmtime (impossible for warp modules anyway), the registry and
  Wasmer Edge deploy (warp deploys to Cloudflare Workers today: `warp deploy`).

## Options weighed
| option | cost | blocking gap |
|---|---|---|
| stay wasmtime only (recommended) | none | none |
| Wasmer as a second runner for `warp self` / WASIX | a second runner with no GC introspection, a WASIX import path, Wasmer-only CI | no GC on any backend |
| switch to Wasmer | rewrite of the runner, gc_traits and host words; lose the component model | no GC, no WIT |

Recheck trigger: a Wasmer release whose `Value` has `AnyRef`/`StructRef`, or features.sh going green on gc_struct.
