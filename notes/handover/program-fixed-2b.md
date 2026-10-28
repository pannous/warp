# program-fixed step 2b: kept values packed (parked 2026-10-28, user decision)

Parked on branch `program-fixed`, not landed: the user stopped the kept blob work. The compiler is to know its
runtime by heart (runtime, prelude and sugar baked into warp.wasm at rebuild, no per-run source or kept files); that
design is still being discussed.

On main already (step 2a, c0ed03fb02): `pack(x)` / `unpack(t)` (core/runtime/packed_nodes.warp), any value as one
text and back. It stays as a general serializer.

On this branch (bd5ff7690f):
- lib/os.warp: keep, kept, keep_of and kept_of go through pack/unpack, under the name `"packed " + name`, so a value
  kept unpacked before is never read as packed.
- packed_nodes.warp writes a class instance as its class's name keyed to its fields, as the host reads one back
  (the compiler keeps lists of instances, e.g. parser.warp library_glyph_operators). Without this the warm run fails
  with "not a list", the message fail() gives.
- Measured: a warm hello 0.21 s -> 0.10 s; `use sound; midi(440Hz)` 2.07 s either way.
- Smoke, self-check and gate tier 1 green on the branch.
