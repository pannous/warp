# E-graphs and equality saturation: what they are, where warp could use one (research, 2026-11-01)

Research only, no code changed. Verdict first: the compiler's sugar is a poor fit (its rules are directional
definitions, not equalities); emitted code is already optimized by Cranelift's own e-graph inside wasmtime; the best
fit is a **user-facing library** `lib/egraph.warp` next to `rules { … }`, ~350 warp lines, no core change.

## 1. What they are

- **E-graph**: a set of **e-classes**, each a set of equivalent **e-nodes**; an e-node is an operator whose children
  are e-classes, not terms. `a*2` and `a<<1` sit in one class, and every parent of that class sees both at once, so
  a graph of a few thousand nodes holds exponentially many equivalent terms.
- **Hash-consing + union-find**: a map `(op, child classes) → class` dedups e-nodes; `union(c1, c2)` merges classes.
- **Congruence closure** (Nelson–Oppen 1980): if `a ≡ b` then `f(a) ≡ f(b)`; a merge can make parents equal, which
  must be merged in turn.
- **Rebuilding** (egg's contribution): instead of restoring congruence after each union, collect dirty classes and
  repair the hashcons and parents in one batch per iteration: much faster, the invariant only holds between batches.
- **Equality saturation** (Tate et al. POPL 2009): repeatedly e-match every rule's left side against all classes and
  union with the instantiated right side, *never deleting*; stop at a fixpoint (saturation) or a limit (nodes,
  iterations, time). Rule order no longer matters: no phase-ordering problem.
- **Extraction**: pick one term per class by a cost function. Tree cost (AST size, op weights) is a cheap bottom-up
  fixpoint; DAG cost (shared subterms counted once) is NP-hard and solved by ILP or heuristics.
- **E-class analyses** (egg): a lattice value per class merged on union, e.g. a constant (`constant folding`), a kind,
  an interval; this is how egg does folding and guards without deleting anything.

Lineage and uses:
- **egg** (Willsey et al., POPL 2021): Rust library, rebuilding + e-class analyses; the de facto tool.
- **egglog** (Zhang et al., PLDI 2023): equality saturation unified with Datalog: relations, functions, lattices,
  incremental (semi-naïve) matching, rules as queries. 1.0 (Oct 2025) moved to a database-first parallel backend;
  current crate 3.0, API `EGraph::parse_and_run_program(filename, text) -> Vec<CommandOutput>`; has a web demo and a
  wasm build example. Successor of egg.
- **Cranelift ægraph** (Fallin, RFC 2022, EGRAPHS 2023): an *acyclic* e-graph mid-end: pure ops float in the graph,
  side-effecting ones keep a skeleton; rules (ISLE) fire once when a node is inserted, union-find gives partial
  canonicalization, no saturation loop, extraction places code back into the CFG. Trades completeness for speed:
  some equalities are never found, but no fixpoint, no cycles. **wasmtime runs it on every function warp emits.**
- Applications: **Herbie** (float accuracy rewrites), **Ruler** (OOPSLA 2021, synthesizes rewrite rules themselves),
  **Tensat** (MLSys 2021, tensor graph superoptimization), **Szalinski** (PLDI 2020, CAD shrinking: loops recovered
  from flat meshes), **SPORES** (VLDB 2020, linear algebra plans), **Diospyros** (ASPLOS 2021, DSP vectorization),
  Cranelift (production JIT). Newer: slotted e-graphs (binders/lambdas), Julia IR (arXiv 2502.17075).

## 2. Where warp could use one

### (a) The sugar / rewrite engine: no, but borrow two ideas
core/passes/rewrites.warp tries rules in file order at each node bottom-up; `at_place` takes the **first** rule that
fits and rewrites its result again (depth 64). Order is meaning, on purpose:
```warp
rewrite $m.$f($a...) => $f($a...) where … $module = module_named($m, $program)     # math.gcd(4, 6)
rewrite $m.$f($k) => (…removed_value…) where … $f in [remove, delete]              # m.remove(k)
rewrite $x.$f($a...) => $f($x, $a...) where $f is symbol and $f not in [add, …]   # xs.f(a)
```
`math.gcd(4,6)` → `gcd(4,6)` and `xs.f(a)` → `f(xs, a)` are *different meanings*, chosen by priority and by context
(`where statement`, `is unbound`, scope). An e-graph would put both readings in one class and let a cost function pick:
exactly the "plausible wrong value" warp refuses (common.md "Refuse, never guess"). Sugar is lowering
(old form → core form), so extraction would always pick the right side: greedy already does that.
The method_calls quadratic (compile_speed.md §1) was a guard reading the whole program per node; an e-graph matches
*more*, so it would cost more, not less. The cost left there (rule tries: rewritten_program 8.7 s + sugar 9.7 s of 166 s)
is cut by **indexing rules by head symbol/operator** (a discrimination net), not by saturation.
Two ideas worth taking: (1) **overlap report**: e-match every pair of sugar rules against each other's left sides
(critical pairs) and list the pairs where two rules fit the same code: those are the order dependences, today
implicit; (2) egglog's semi-naïve matching: only retry rules on nodes that changed.

### (b) Exact arithmetic: normal forms beat saturation
lib/reals.warp keeps `c·√r·π^k` in a canonical form; `√2 + √3` falls to float (`alike` fails). The cure is a richer
normal form (sums over a basis of square-free radicals: `(√2+√3)² = 5 + 2√6`), which is decidable and cheap.
Units are dimension vectors, also canonical. E-graphs earn their keep only where no canonical form exists: symbolic
code with commutativity/associativity plus identities (trig, logs), i.e. in (f). One small extraction use: printing a
derived unit by its shortest spelling (`kg·m²/s²` → `J`), a lookup, not an e-graph.

### (c) Optimizer over emitted code: mostly done by wasmtime already
Constant folding, GVN/CSE, strength reduction, LICM on the wasm warp emits are Cranelift's ægraph's job at load time
(and V8's/SpiderMonkey's in the browser). A warp-level copy would duplicate it. What Cranelift cannot see are warp's
runtime calls: stage two's profile (record 79a8a80d282e6c60) shows `is_class_instance` 6.6%, `node_equal` 4.9%,
`node_text` 4.0%, `int_node` 2.2%, GC ~25%: boxing and dynamic kind tests. Removing those is **kind specialization**
(knowing `x: int` to emit `i64.add` instead of `node_add`), a lattice fixpoint over facts: egglog's Datalog half
fits that shape (see e), the equality half adds little. Cheaper first measurement: binaryen `wasm-opt -O3 --gufa`
on warp.wasm, compare stage-two instructions retired.

### (d) warp format / beautify: normalizing rules suffice
`warp format` is layout (meaning kept by construction); `beautify`'s synonyms-canonical step applies "I meant" fixes,
each kept only if the lowered program stays the same. Every rule there is directional (alias → canonical word), so
the result is unique without extraction. An e-graph would matter only if beautify grows *competing* best-practice
rewrites (`if x then yes else no` → `x`, `not (a < b)` → `a >= b`, `xs.map(f).filter(g)` vs a comprehension)
where the shortest/canonical form needs a cost. Not today.

### (e) Type and overload resolution: union-find yes, saturation no
Unifying type variables is congruence closure on types: half an e-graph. Overload resolution (`|` on types vs
functions vs values, decision rules-to-typed-operators) and the emitter's "one widening round" of kinds are fixpoints
of facts over a lattice. egglog's paper does type inference this way. Fit: a principled rewrite of analysis/, i.e. a
redesign, not an experiment; arity_overloads 6.2 s is mostly `nodes_in`, not resolution.
```warp
infix operator |(a: type, b: type) := a | b        # text | int: a union
infix operator |(x: any, g: function) := g(x)      # 3 | inc: a call — dispatch, not equality
```

### (f) `rules { … }` and tree patterns as a user-facing e-graph: the best fit
`rules {…}` (decision rewrite-rules) is a terminating normal-form rewriter; it **cannot hold commutativity**: the rule
loops until the 1000-step cap names it. An e-graph holds it, and associativity, and finds the cheapest form:
```warp
use egraph
# an equality is a pair [left, right], applied both ways: plain lists, no new syntax
algebra = [
	[data($a * $b), data($b * $a)]
	[data(($a * $b) / $c), data($a * ($b / $c))]
	[data($a / $a), data(1)]
	[data($a * 1), data($a)]
]
simplify(data(2 * y / 2), algebra)          # data(y)   — `rules {…}` loops on the first rule
simplify(data(x * 0 + y), algebra + zero)   # data(y)
equal(data(a*(b*c)), data((c*b)*a), algebra) # yes: a proof by saturation
```
(`$a / $a = 1` holds only for a ≠ 0: such a guard is an e-class analysis, e.g. a known-nonzero constant, as egg does.)
Users get symbolic algebra, Herbie-style float rewrites (`sqrt(x+1) - sqrt(x)` → `1 / (sqrt(x+1) + sqrt(x))`),
units/expression simplification for `data(…)` code, plus `find`/`replace` modulo equalities, with no core change.

## 3. Costs and risks
- **Blow-up**: assoc + comm alone generate exponentially many forms (all bracketings × orders); saturation rarely
  reaches a fixpoint on real rule sets. egg's default runner stops at 10 000 e-nodes, 30 iterations, 5 s; Tensat
  needed explicit node limits; Cranelift avoided the loop entirely. Every use needs limits and a loud "stopped at the
  node limit" note (not a silent best guess).
- **Extraction**: tree cost is linear; DAG-optimal is NP-hard (ILP); a library starts with tree cost.
- **Compile time**: the compile was just cut from 820 to 166 s CPU (stage two 2efb9b98f2f3eca9; latest record
  79a8a80d282e6c60: 192.8 s user, 3.39·10¹² instructions retired, 575 MB). Anything per node across the compiler
  (an e-graph of the whole program) reintroduces the superlinear terms compile_speed.md removed. Hence: not in the
  compiler's hot path.
- **Minimal e-graph in warp**: union-find `parent: [int]` ($IntList, i64 items; GC arrays grow by copy, so amortized
  doubling); e-node `class ENode { op: text, args: [int] }` (op the Node's symbol/operator/literal text; children class
  ids); hashcons a `$NodeMap` keyed by `op + "(" + args.join(",") + ")"` (a text key costs allocation; a later step
  hashes to int); per class `nodes: [int]`, `parents: [int]`, `analysis` (constant or ø). Size: add/find/union/rebuild
  ~150 lines, e-matching of `data(…)` patterns with `$` holes ~80 (reuse the `matches` semantics of rules values),
  run loop with node/iteration limits ~40, tree-cost extraction ~50, Node ↔ e-graph ~40: **~350 lines**. Teaching
  implementations in other languages are 200–400 lines; egg's core is a few thousand.
- **egglog as a library natively**: the crate adapter (notes/crate_adapter.md, `scripts/warp-crate build`) wraps any
  Rust crate as a C library over handles and text; a manifest of ~5 lines would do:
  ```warp
  crate: "egglog@3"
  egglog_new(): handle(egglog::EGraph) from 'rs_egglog' = egglog::EGraph::default
  egglog_run(e: handle, program: text): text from 'rs_egglog' = …parse_and_run_program  # needs a small adapter: Option arg, Vec<CommandOutput> → text
  ```
  Costs: a large dependency tree (parallel backend), a text interface (warp builds egglog s-expressions and parses
  outputs back), and a second notation for rules. In the browser: egglog builds to wasm (repo wasm-example, the web
  demo), so the playground could load it as a JS-side module answering the same host words; unverified whether the
  3.x parallel backend builds single-threaded for wasm32 without changes. Either way two implementations behind one
  API; a pure-warp library runs identically in both and stays inspectable. **Recommend pure warp first; egglog only if
  scale demands it.**

## 4. Recommendation: one experiment
**lib/egraph.warp, the user-facing library of (f)**: `simplify(code, rules, limits)` and `equal(a, b, rules)` over
`data(…)` patterns, ~350 lines plus `test/egraph/test_egraph.pairs.warp` (~15 lines). Pure lib/: smoke test to merge,
no core OK, no compiler build, and it reuses the pattern machinery users already know.
Measure:
- value: the test folder's cases that `rules {…}` cannot do (comm/assoc loop to the cap, order-dependent normal
  forms) and that `simplify` does: count passing, e.g. `simplify(data(2*y/2), algebra)` → `data(y)`;
- cost: `/usr/bin/time -l warp test test/egraph` instructions retired and e-nodes/iterations reported per case, at
  growing input sizes (2/4/8/16 terms of a commutative sum) to see where the node limit binds;
- the compiler is untouched, so stage-two instructions (3.39·10¹² in record 79a8a80d282e6c60) must not move.
Follow-up only if that shows value: the **sugar overlap report** of (a) built on its e-matcher (lists pairs of
lib/sugar rules fitting the same code, offline, no compile-time cost), and a head-symbol rule index for the 18 s of
rule tries, measured as stage-two instructions retired before/after (scripts/self-check.sh record `time`).

## Sources
- Willsey et al., *egg: Fast and Extensible Equality Saturation*, POPL 2021: https://arxiv.org/abs/2004.03082, https://egraphs-good.github.io/
- Zhang et al., *Better Together: Unifying Datalog and Equality Saturation* (egglog), PLDI 2023: https://arxiv.org/abs/2304.04332
- egglog crate, changelog, web demo, wasm example: https://crates.io/crates/egglog, https://docs.rs/crate/egglog/latest/source/CHANGELOG.md, https://github.com/egraphs-good/egglog
- egglog `EGraph` API: https://docs.rs/egglog/latest/egglog/struct.EGraph.html
- Tate et al., *Equality Saturation: a New Approach to Optimization*, POPL 2009: https://arxiv.org/abs/1012.1802
- Nelson & Oppen, *Fast Decision Procedures Based on Congruence Closure*, JACM 1980
- Cranelift e-graph RFC: https://github.com/bytecodealliance/rfcs/blob/main/accepted/cranelift-egraph.md; prototype https://github.com/bytecodealliance/wasmtime/pull/4249
- Fallin, *ægraphs: Acyclic E-graphs for Efficient Optimization in a Production Compiler*, EGRAPHS 2023: https://cfallin.org/pubs/egraphs2023_aegraphs_slides.pdf; blog https://cfallin.org/
- Bernstein, *What's in an e-graph?*: https://bernsteinbear.com/blog/whats-in-an-egraph/
- Panchekha et al., *Herbie*, PLDI 2015: https://herbie.uwplse.org/
- Nandi et al., *Rewrite Rule Inference Using Equality Saturation* (Ruler), OOPSLA 2021: https://arxiv.org/abs/2108.10436
- Yang et al., *Equality Saturation for Tensor Graph Superoptimization* (Tensat), MLSys 2021: https://arxiv.org/abs/2101.01332
- Nandi et al., *Synthesizing Structured CAD Models with Equality Saturation and Inverse Transformations* (Szalinski), PLDI 2020: https://arxiv.org/abs/1909.12252
- Wang et al., *SPORES: Sum-Product Optimization via Relational Equality Saturation*, VLDB 2020: https://arxiv.org/abs/2002.07951
- VanHattum et al., *Vectorization for Digital Signal Processors via Equality Saturation* (Diospyros), ASPLOS 2021: https://dl.acm.org/doi/10.1145/3445814.3446707
- *Equality Saturation for Optimizing High-Level Julia IR* (notes Cranelift's incompleteness): https://arxiv.org/abs/2502.17075
- warp: notes/compile_speed.md, core/passes/rewrites.warp (at_place, rewritten_again), lib/sugar/method_calls.warp,
  lib/reals.warp, notes/crate_adapter.md, notes/decisions.md (rewrite-rules, rules-to-typed-operators), stage-two
  record /Volumes/Build/compilers/79a8a80d282e6c60/stage-two.record
