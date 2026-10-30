
## hello compile RSS split (2026-10-30, card hello-compile-rss, vmmap at exit of `warp samples/hello/hello.warp`)
- physical footprint 173.8M (peak 177.6M); RSS 306M incl. ~110M clean shared dyld-cache pages (__LINKEDIT, __OBJC_RO, libs)
- compiler GC heap (512M reservation at 0x7202000000): 166.8M touched + dirty = 96% of footprint
- compiler cwasm (warp.wasm.driver.cwasm, 36M mapped, file-backed COW clean): 22.7M resident, 0 dirty
- program's GC heap (64M): 16K touched; malloc zone 4.4M; stacks 0.3M; warp __TEXT 9.9M clean
- so wasmtime itself and the code are negligible; the cost is the compiler's GC garbage never collected below 512M
- driver-side collection has no trigger: a hello compile makes 0 host_word calls (lldb hit count); the compiler's only imports
  in use are WASI (fd_read/path_open), answered inside wasmtime, never by driver.warp
- wasmtime 50's copying collector is a Cheney semi-space without madvise: a collection flips allocation into the idle half,
  touching fresh pages, and gives nothing back; gc_heap_initial_size is a Tunable, so it is in the cwasm compatibility check
- compiling-hello phase probe (warn markers + lldb footprint at each write; no collection, so growth = allocation), MB:
  startup 13.3 | implicit uses + C functions 5.8 | prepend_runtime+relocate+shake 24.7 | parse+source checks 13.5 |
  with_uses..used modules 6.4 | with_reached prelude 39.1 | 28 pure passes 44.5 (5 sugar-rule passes 22.2: sugar 6.2,
  late 5.0, iteration 4.7, comprehension 3.3, first 3.0; lambdas 4.4) | called_only 1.1 | emit 29.4 | total 177.6
