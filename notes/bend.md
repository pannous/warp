# Bend, HVM and what warp takes from them

Bend 2 (github.com/bendlang/bend) no longer runs on interaction nets: it is a Python-shaped language with an affine
dependent type theory (BendTT, proof kernel in Lean), laws + proofs, compiled to C (CPU threads, Metal, CUDA) or JS.
Its runtime BendRT has no GC (a `match` frees the node it consumes, `+` values are reference counted), no C call stack
(every call a tail, a cut or a fork) and a fork-join "cube" scheduler without work stealing. The HVM line (HVM1–HVM4,
interaction calculus) is a separate research runtime. For warp the value is in the ideas, not the code.

## Taken
- **Parallel let** `a b = f(x) g(y)` (src/lowering/control/parallel.rs, notes/go_blocks.md "Parallel let"): lowered
  onto go/await, the last call on the current thread, forks only down to a depth cutoff (BendRT: a fork met while
  draining runs as a plain call).
- **Laws convention** (notes/laws.md): `for x: T` quantifier lines on named laws, samples/laws/project_laws.warp is the
  human-owned contract agents must not edit, `warp verify` exits successfully only when every law is proved ("all
  proofs check / some proofs fail"). Warp checks generated Lean directly, so no `PROOF.warp`.

## Not taken (checked)
- A second backend to HVM2 or Bend 2: frozen 24-bit runtime / U32-F32-Nat numbers, full annotations, affine values and
  termination; warp's strings, maps, big ints, units, IO, FFI and mutation don't map.
- Porting an HVM kernel to wasm linear memory (shared-memory parallelism): HVM4 is unlicensed, a second value model
  beside $Node.
- BendRT's allocator: WASM GC already frees; the cube needs shared memory, which GC objects lack.
- Interaction-net evaluation (Bend itself dropped it), affine-only discipline.

## Candidates
- Proofs as warp defs for recursive laws (equality, rewrite, induction by a recursive call), checked by the Lean export.
- Export pure annotated functions + laws to `LAWS.bend` for `bend PROOF.bend --verdict` as a cross-check.
- A fixed-pool fork-join scheduler for `go` instead of a thread per task.
- Longer term, an own proof kernel: notes/own_prover.md.

## Licenses (if code is ever copied)
- Apache-2.0 (Bend 1+2, HVM2): porting allowed; ship the license, keep the copyright line, mark modified files, carry a
  NOTICE if one appears; no trademark use beyond attribution. MIT (HVM1, HVM3, Kind): keep the notice.
- HVM4 has no license: read and re-implement published rules only, never copy.
- Ideas, syntax and semantics need no attribution; credit Bend in the wiki anyway. Mechanics: `THIRD_PARTY/bend/LICENSE`
  + `THIRD_PARTY/README.md` (file → origin commit) and a header comment per ported file.
