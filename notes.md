
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
- compiling-hello reach probe: with_reached(prelude) looks up 210 wanted words and reaches only 2 prelude slices, yet the
  loop allocates 36 MB (~30 ms): each word scans every slice (`for slice in slices { if slice#1 == word …`), ~860 B per
  comparison. So prebuilding the prelude can't help hello; an index of the slices by name can. (lldb stops cost ~7 ms
  each, so per-pass ms above are inflated: 28 passes ≈ 35 ms real, 45 MB)
- compiling-hello DONE (c4d15e63ed): slice_origins re-read the prelude and counted every slice's line (34.6 MB, ~29 ms);
  now baked_origin per reached slice from its key + LIBRARY_FILES: hello peak footprint 228.5 -> 191.2 MB, RSS 257 -> 220.
  Probe kit: data/rss/footprint.py + phases.cmds (lldb break on write, footprint per warn marker; ~7 ms per stop).
  Next big ones: emit 29 MB, runtime prepend 25 MB, 28 passes 45 MB (sugar-rule passes half)
- hello-compile-allocations probe (worktree runtime-prepend): runtime.wasm read 16.4 MB (232 KB file: ~70 B per byte,
  each byte a boxed Int Node + i64box via text_byte_list), warp.runtime decode 8.2 MB, module_rules_of 1.6 MB/24 calls
  (no disk read without WARP_LIB: file_text = parser.warp file_source). Int boxing has one site, emitter.warp:626;
  Node fields are immutable, so small Int nodes could be shared (shape sent)
- shared small ints landed (194b3d1e70, runtime nodes.warp int_node, -128..1023): hello 192.6 -> 176.2 MB, read_bytes of
  runtime.wasm 245.7 -> 214.8. Traps: runtime files take no top-level constants (zero-argument words instead); a Node
  global's zero value is a constant expression, so module_layout.warp zero_code_of keeps fresh_int_code, no call
- after shared ints: runtime.wasm read 9.4 MB (was 16.4), warp.runtime decode 6.9 MB (was 8.2); lazy decode (shape B)
  recommended dropped: 23 module.runtime sites for < 7 MB
- definition-slices (845734129f): prelude_definitions() (83ab6595ed) cut the whole prelude anew whenever no earlier
  file_slices call had filled source_slices (order-dependent; shebang `env` made os a candidate in samples/hello).
  definition_slices now takes baked LIBRARY_SLICES for any bundled text, file_slices folded in. Compile peak footprint,
  WARP_LIB on both sides: hello.warp ("hi") 190.4 -> 143.8 MiB, samples/hello 232.0 -> 151.1 MiB. O(n²) cut of
  unbundled text: card definition-slices-quadratic
- shared subtrees (walk-children-rebuild): a pass's walk returns n itself when every child came back same_node; fresh
  hello unchanged at ~218 MiB RSS; pure rebuild 664 s, 4.86 GB RSS. The peak was not the compiler (flat ~1.13 GB) but
  the native driver never freeing a run: store_delete never hit, footprint 0.53 -> 1.38 -> 4.23 -> 4.81 GB per run
- rebuild-pure-peaked (e7cb00f317): run() deletes store, linker, module, engine after each run: rebuild peak
  4.86 -> 2.28 GB RSS, 651 s unchanged. Precompiling warp.wasm (Cranelift) alone peaks at 2.62 GB
- compile heaps: the semi-space GC never collects a compile that fits its initial heap, so the heap size IS the RSS.
  heap is a Tunable baked into machine code: one cwasm per heap (.driver.cwasm 64 MB, .driver.build.cwasm 512 MB for
  compiler.warp). hello s / RSS MiB: 8 MB 0.50/59, 16 MB 0.41/53, 32 MB 0.21/69, 64 MB 0.20/101 (was 218 at 512);
  sudoku 2.70/2.28/1.43/1.21 s, footprint ~78 MB at every size. 3.8 MB warp.wasm -> 39.5 MB cwasm (card wasm-becomes)
- module-imports (550dee2568): full.wasm imports only host words its code reaches, unreached runtime callers go to one
  trapping stub per type: hello 81 -> 1 imports. Trap: the seed keeps the FIRST of two defs of a word silently (a
  duplicate runtime_import_entries dropped the type byte; card compiler-word-redefinition); map int .keys with < traps
