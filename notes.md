
## hello compile RSS split (2026-10-30, card hello-compile-rss, vmmap at exit of `warp samples/hello/hello.warp`)
- physical footprint 173.8M (peak 177.6M); RSS 306M incl. ~110M clean shared dyld-cache pages (__LINKEDIT, __OBJC_RO, libs)
- compiler GC heap (512M reservation at 0x7202000000): 166.8M touched + dirty = 96% of footprint
- compiler cwasm (warp.wasm.driver.cwasm, 36M mapped, file-backed COW clean): 22.7M resident, 0 dirty
- program's GC heap (64M): 16K touched; malloc zone 4.4M; stacks 0.3M; warp __TEXT 9.9M clean
- so wasmtime itself and the code are negligible; the cost is the compiler's GC garbage never collected below 512M
