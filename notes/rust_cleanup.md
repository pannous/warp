# Rust cleanup: the lessons of the self-hosting rewrite, applied to the Rust compiler first
Roadmap card: rust-cleanup (board).

Why: the Rust compiler stays as fallback and reference implementation (notes/self_hosting.md). Cleaned up first,
it gives the port a clean basis, so the warp stages don't meet the same historic detours again.

How: an intermediate phase. Every self-hosting worker finishes its current card, then does the Rust side of the
structural decisions in its own stage (one stage per worker keeps conflicts low), then self-hosting resumes and
ports the cleaned design. The scope is the decided restructurings below, not an open-ended refactor. The measure is
src/ shrinking (each step states its net src/ delta) while the suite stays mostly green: a cleanup branch may break
up to 10% of the tests when they are edge cases (rare forms, historic detours), never core behaviour. The branch
lists the tests it breaks in notes/known_failures.md, one line each `- <module::test> | <card> | <reason>`, in its
own commits (push_tested.sh accepts exactly those); the Integrator accepts it, records them as known failures so
other branches aren't blamed, and on demand turns them into cards for the stage's worker.

| stage | worker | steps (cards) |
|---|---|---|
| parser | warp-parsing | written operators kept, desugaring in one sugar pass (S1, parser-desugar-pass); scope-aware call vs words (S2/S3, parser-scope-pass, parser-application); tokens with blank/newline bits (S4, parser-tokens); typed fields to the type pass (S5); one context value (S6); one keyword-form table (S7); rust-square-cube, rust-definition-head, rust-bare-return, rust-colon-body; function-words-consume |
| lowering | warp-self-passes | the target pipeline (notes/self_hosting.md): repair passes dropped once the parser is clean, one table-driven word-phrase pass (of_lookups, member phrases, times phrases, number words), checks moved to the analyzer; rust-renames |
| analyzer | warp-self-analyzer | the flow inference of analyzer.warp replaces the Rust heuristics (src/analyzer kinds), builtin kinds read from lib signatures |
| emitter | warp-self-emitter | rust-emitter: typed values inside, $Node only at the edges, GC byte-array texts, vector lists; runtime words as lib warp through the pipeline (runtime-words) |
| library | warp-self-survey | retire-library-words: built-in words as ordinary lib functions, the library_words.rs tables gone; the library sweep continues |
| patterns | warp-self-splice | the suite runner first (warp-pipeline-suite), then code-patterns and the pattern roadmap (notes/matching.md) |

Order inside the phase: the parser first (later stages read its trees), the others in parallel where they don't
depend on it. Branches already Ready are merged before the phase starts.

## The board's own cleanup cards
Each worker also sweeps the open cards of its stage's topics (Next, Soon, Later) and takes those the cleanup touches
anyway: parser → parsing, parser-features, lookup; lowering → units, modules, effects; analyzer → types,
type-theory, errors; emitter → printing, codegen, format, runtime-speed; library → the self-hosted library moves.
New feature gaps (spec-gaps) stay where they are.

## Warp files in the Rust chain
Where a warp part is finished and clean (lib/compiler passes such as number_words.warp, times_phrases.warp,
interpolation.warp, sugar.warp; library words already moved to lib/*.warp), the Rust pipeline runs it instead of
keeping a Rust twin, and the Rust twin is deleted. Condition: compile time measured before and after (each pass
runs as compiled warp at compile time); a pass that costs too much keeps its Rust form until the prelude parsing
and caching work makes it cheap. This is the first real use of the warp compiler parts and shrinks src/ for good.

Measured 2026-10-18 (warp-self-passes, times_phrases.warp with parser.warp, debug build): compiling the pass with
its bridge takes 25 s CPU (41 s wall), the compiled module is 228 KB, and one run of it (wasm compile + a tiny
program) 0.6 s. A pass at compile time would need the module built ahead and embedded (build.rs would need a warp
compiler: the bootstrap problem) and still costs ~0.5 s per pass and program. So the Rust twins stay for now; this
comes back with AOT-embedded compiler modules (cwasm) and the prelude caching work.

## Lowering: helper dedup (warp-self-passes, 2026-10-18)
Landed or Ready: rust-scope (program_names::Bindings, one walk for the names a program gives, -31) and rust-dedup
(-60): Node::written_names (`x` of every `x = …`/`x := …`), Node::ungrouped (`(x)` → x under its meta; word_help keeps
its own because a non-group keeps its position there), `node.symbol_name().and_then(…)` for two-arm symbol matches.
What a duplicate scan of src/ found: no two functions with identical bodies; the near-twins left are deliberate
(serve.rs and broadcasting.rs `replaced` compare by serialize vs by node). Functions that recurse into `(x)` themselves
convert to `match x.ungrouped()` only when no arm reads the parameter again (guards or bodies would see `(x)` instead
of x); the ~8 left (type_names, law expressions, for_loop, time holds_time, equality, control_flow) need a reading of
each arm's meta use first. The lowering is otherwise well factored: the remaining line gains are the repair-pass
drops after parser S2/S3 and the word-phrase pass, not helpers.
