# simd128 map benchmark (card simd-map)

`map_bench.wat`: y = x*0.5 + 1 over N floats, R rounds, 1e9 items in all. Run:

    wasmtime run -W gc=y,function-references=y --invoke <function> map_bench.wat
    wasm-tools parse map_bench.wat -o ../../scratch/map_bench.wasm && node bench.mjs   # V8

Functions: scalar_gc (a new GC f64 array per round, today's shape), scalar_gc_in_place, simd_copied (GC array copied to
linear memory, f64x2 map, copied back into a new GC array), scalar_memory, simd_memory (f64x2 in linear memory).
Results: notes/simd.md.
