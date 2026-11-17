# What else to tweak in our wasmtime (2026-11-13, session warp-wasmtime-tweaks)

We bundle and patch wasmtime 50 (crates/wasmtime-capi, patches/*.patch, scripts/wasmtime-c-api.sh). The model is
subtype-display.patch (notes/compile_speed.md "Emit"). This note measures where wasmtime's own code still costs time
and ranks the remaining changes.

## How it was measured
- Profiles come from `sample` at 1 ms, named through the --profile perf map (scripts/profile.sh's parser). Stage 3
  used the store's own record (<store>/47275c9ad3defa82/warp-self.record, sampled at 10 ms).
- The A/B runs use a cloned install (`cp -c -R` of the native build) whose libwarp_wasmtime.dylib is a prototype
  build. Its config setters read PROTO_* variables, for the prototype only, never landed. Each figure is the
  instructions retired (`/usr/bin/time -l`), the lower of two warm runs. Cycles are given where the two disagree.
- The stage 3 command is `warp --lib <clone> rebuild self`. Typical programs: `print 1`; a number loop
  (`s = s + i % 7`, 150 M); text iteration (`for c in t`, 4 M chars); closures (`sum(map(xs, x => x*k % 3))`, 1 M);
  samples/gpu/julia.warp; the console fed 300 lines.

## Where wasmtime's own code costs time

| path | stage 3 | console (3000 lines) | typical programs |
|---|---|---|---|
| copying GC (gc_alloc_raw → do_gc → collect_increment: scan_field, forward) | 8.4% | 10.2% | ≤0.1% |
| ref.func libcall (ref_func → get_func_ref → initialize_and_return_funcref) | 1.8% | 1.4% | 0 |
| funcref fields of GC structs (get_interned_func_ref 1.1%, intern_func_ref_for_gc_heap 0.8%) | 1.9% | small | 0 |
| compiling the new warp.wasm (compile_module, ~120 G instructions cold) | 3.0% wall | 1.5% | 0 |
| is_subtype | gone (subtype-display) | – | – |
| cwasm deserialize | – | – | 0.3% |

For typical programs, wasmtime's own code is noise. Their time goes to warp's runtime words: int_node, exact_int,
sum_overflows. The number loop costs about 284 instructions per `s + i % 7`, and that cost is in warp, not wasmtime.

## Measured candidates, ranked

| # | change | size | gain (instructions) | risk | card |
|---|---|---|---|---|---|
| 1 | builder GC heap 1 → 2 GiB (BUILDER_GC_HEAP_BYTES, driver.warp) | 1 line | stage 3 −4.2% (2.657 → 2.544 T), cycles −7.7% | peak memory 1.09 → 2.16 GB | builder-heap-2g |
| 2 | ref.func as an address (patch below) | 37 lines, 2 crates | stage 3 −2.3% (→ 2.596 T), console −1.2%, start +0.3% | low: instantiation fills every escaping VMFuncRef | ref-func-inline |
| 3 | funcref fields without libcalls | ~60 lines, 3 crates (or emitter: table index + call_indirect) | ≤1.9% stage 3 | medium | funcref-field-libcalls |
| 4 | Cranelift inlining (Inlining::Yes) | 3 lines | stage 3 −8%, text −17%, closures −18%, console −9% | **cycles up**: stage 3 +10%, loop 2.5× slower | wasm-inlining (note) |
| 5 | constant words emitted in place (small_int_low/high are calls) | emitter | loop.warp ~7.7% of own time | low | constant-words-inline |

The three together (1 + 2 + inlining) reach stage 3 2.271 T instructions (−14.5%) but 0.631 T cycles (+5%): inlining
eats the rest.

## Tried, no gain
- cranelift_opt_level speed_and_size: within ±0.1% everywhere.
- gc_heap_may_move(false): 0%. Bounds checks on GC fields are already elided: the GC heap is a 4 GiB reservation with
  a 32 MiB guard, and the disassembly shows no check on struct.get.
- RUN_GC_HEAP_BYTES ×4 for programs: console −1.2%, julia −2.3%, the rest flat or slightly worse; not worth the memory.
- Epoch checks: epoch_interruption is never on (uncached_config passes `no`). Fuel: only for `use fuel` / `warp fuel`.
- cwasm load: 0.3% of a run (mmap).
- Not measured: regalloc single-pass. It only shortens the cold compile of a new warp.wasm (~4.5% of a fresh stage-3
  run) and makes slower code.

## Why inlining costs cycles
The instruction count falls, but loop.warp's cycles go from 7.4 to 19.0 G even when only callees of ≤12 bytes are
inlined. The profile moves into raised_if_error (251 → 2456 samples). Once its callee `raised` is inlined, its prologue
saves all 20 callee-saved registers (x19–x28 and d8–d15) and its epilogue restores them on every call, including the
fast path, which only reads one field. A throwing callee drags the clobber-everything ABI into a hot caller.
Inlining becomes worth a retry once should_inline keeps callees that can throw out (a wasmtime patch), or once
Cranelift shrink-wraps the saves. A cold inline also costs compile time and memory: ~197 vs ~120 G instructions for
warp.wasm, and peak RSS 7.4 GB. The old blocker, error names found by trap frames, is gone: the driver names errors
by raised_message.

## Prototype: ref.func as an address (card ref-func-inline)
Bump WARP_VERSION's suffix in config.rs alongside it, so older machine code is compiled anew. The comment above
initialize_and_return_funcref upstream invites exactly this change "when ref.func becomes more commonly used". Warp
uses ref.func for every closure and every class word.

```diff
--- wasmtime/src/runtime/vm/instance.rs (after initialize_vmctx in Instance::new)
+        // warp: every defined function a ref.func may name gets its VMFuncRef now, so compiled code takes its address
+        let module = ret.get_mut().env_module().clone();
+        for (index, func) in module.functions.iter() {
+            if let (Some(defined), false) = (module.defined_func_index(index), func.func_ref.is_reserved_value()) {
+                ret.get_mut().initialize_defined_funcref(req.store.modules(), defined);
+            }
+        }
--- wasmtime-internal-cranelift/src/func_environ.rs (translate_ref_func)
+        // warp: a defined function's VMFuncRef is filled when the instance is made, so ref.func is its address
+        if self.module.defined_func_index(func_index).is_some() {
+            let func_ref = self.module.functions[func_index].func_ref;
+            if !func_ref.is_reserved_value() {
+                let vmctx = self.vmctx_val(&mut pos);
+                let offset = self.offsets.func_refs().at(func_ref);
+                return Ok(pos.ins().iadd_imm(vmctx, i64::from(offset)));
+            }
+        }
```
The prototype ran stage 3 to its end (the compiler compiled itself) and every typical program, all with the same
output. It still needs the crate's tests (tests/subtype_display.rs as a model) and a check that table-initialized
funcrefs still compare equal (ref.eq) to ref.func ones. Both share the same VMFuncRef slot, so they should.
