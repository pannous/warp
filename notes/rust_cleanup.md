# Rust cleanup: the lessons of the self-hosting rewrite, applied to the Rust compiler first
Roadmap card: rust-cleanup (board).

Why: the Rust compiler stays as fallback and reference implementation (notes/self_hosting.md). Cleaned up first,
it gives the port a clean basis, so the warp stages don't meet the same historic detours again.

How: an intermediate phase. Every self-hosting worker finishes its current card, then does the Rust side of the
structural decisions in its own stage (one stage per worker keeps conflicts low), then self-hosting resumes and
ports the cleaned design. The scope is the decided restructurings below, not an open-ended refactor. The measure is
the full Rust suite staying green while src/ shrinks; each step states its net src/ delta.

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
