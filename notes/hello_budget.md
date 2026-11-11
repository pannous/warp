# What `warp hello.warp` costs (card hello-warp-retire, 2026-11-10)

`print "Hello"` retires ~1.58 G instructions in 0.15 s (macOS `/usr/bin/time -l`, 75 MB footprint); `/bin/echo` retires
~9 M. Phases below: instructions from `/usr/bin/time -l` on whole runs, shares inside the compile from
warp-test-loop's sampled profiles of 15 one-liners (~1200 samples, ~/Library/Logs/warp/warp-test-loop/prof/,
card compile-setup-overhead), wall marks from `--times` (which overstate setup: they are wall spans).

| phase | costs | minimal need | over 10x? | card |
|--|--|--|--|--|
| process, dyld, wasmtime engine, warp.wasm's machine code mapped (`warp version`) | 145-180 M | ~10-20 M (echo 9 M + one mmap) | yes, ~10x | version-retires |
| implicit uses + C functions + prepend_runtime | <1% each sampled (--times 23+28 ms are wall gaps) | ~1 M | no | - |
| linking the prelude (linked_libraries + link_libraries) | ~8%, ~125 M | read a packed signature, ~5 M | yes | compile-setup-overhead 1) packed prelude signature |
| lowering passes over the program + its reached module words (checked_program / lowered_reached) | ~45%, ~700 M | a one-call program: ~5 M | yes, >100x | sugar part: compile-setup-overhead 2); the rest: non-sugar-lowering |
| emit (typed_wasm_module) for a 3 KB module | ~10%, ~160 M | ~2 M | yes | kb-costs |
| GC heap start | in the above; RSS 75 MB | a few MB | measured only | compile-setup-overhead 3) |
| Cranelift on the program's module | ~12% when the program is new; 0 when repeated (wasmtime's cache: driver.warp warp_config) | 0 when cached | no | - |
| the run | `warp hello.wasm` cached ~50 M, `./hello` ~42 M, of which ~40 M is startup | ~10 M | no | - |

Sizes: hello compiles to hello.mod.wasm 3,168 bytes (the program apart, importing runtime and prelude);
hello.wasm 39,469 (shaken whole module); hello.full.wasm 217,151. The program module does not carry the runtime.
A first `warp hello.wasm` costs 1.28 G (Cranelift on the 39 KB whole module), later ones ~50 M.

So the compile itself is ~85% of the hello cost, and inside it the fixed per-pass and per-emit cost on a
tiny program dominates: about 60 passes each walk the program and the module words it reaches, and emit sets up
its tables, whatever the program's size. compile-setup-overhead's three fixes cover the prelude link and the sugar
walks; the per-pass walks of the non-sugar passes, emit setup and engine startup are carded separately.
