# Warp semantic architecture

## Purpose

Warp is already more than a prospective language design. It has a flexible data/code
syntax, a recursive-descent parser, basic type analysis, user-defined records and
functions, direct WebAssembly GC generation, runtime integration, and end-to-end
round-trip tests. The next step should therefore not be a second compiler or a large
syntax expansion. It should be a semantic boundary inside the compiler.

The intended direction is:

```text
source text
    -> lossless structural Node tree
    -> resolved, typed semantic IR
    -> ownership/effect/cost plans
    -> WebAssembly GC
```

`Node` remains the universal data representation and interchange format. It should no
longer also be the compiler's final semantic representation.

## What already fits the design

### A compact, expressive surface already exists

The parser and operator model already support many of the desired ideas:

- concise definitions with `:=`, ordinary assignment with `=`, and typed forms with
  `:`;
- several function-definition spellings, plus implicit `it` parameters;
- word operators such as `and`, `or`, `not`, `if`, `then`, `else`, `while`, and `to`;
- Unicode alternatives such as `≤`, `≥`, `≠`, `√`, `²`, and `³`;
- calls with optional-looking punctuation, blocks, lists, records, tags, and rich data;
- a normalization-hint layer that accepts multiple spellings while recommending a
  configurable canonical view.

This is a concrete version of "one semantic model, several resolutions." The
normalizer is especially valuable: it means accepting a friendly surface does not
require making every spelling canonical.

### Structural data is a genuine foundation

`src/node.rs` defines a small universal tree whose `Key`, `List`, `Type`, and `Meta`
forms represent both data and parsed programs. That matches the idea that code should
be inspectable data and gives Warp a natural serialization and macro substrate.

The three-field WebAssembly `$Node` representation preserves this model at runtime,
while user-defined `Type` declarations can also become named WebAssembly GC structs.
This combination is unusually useful: open structural data can coexist with efficient
nominal layouts.

### The compiler has the beginnings of elaboration

`src/analyzer.rs` already performs several early semantic passes:

- scoped local collection;
- coarse expression-kind inference;
- reassignment compatibility checks;
- function extraction from multiple surface forms;
- implicit `it` discovery;
- type and FFI collection;
- required-runtime-function discovery for tree shaking.

`src/context.rs`, `src/function.rs`, and `src/type_kinds.rs` already contain registries
that can seed a real symbol/type environment. The emitter performs forward collection
of types and function signatures before compiling bodies.

### WASM-first is an asset, not a constraint to route around

`src/wasm_emitter/` already owns a substantial direct WebAssembly GC backend with named
types, constructors, functions, globals, strings, FFI/WASI integration, and runtime
round trips. Generating Rust first would postpone the most important research question
and discard information into Rust syntax only to recover it in `rustc`.

Keep direct WASM as the initial production backend. If useful, add a tiny IR
interpreter as an executable specification and differential-test oracle. A Rust
backend may later be useful for interoperability or debugging, but it should not
define Warp semantics.

## The current architectural limit

Today the same `Node` shapes are interpreted repeatedly and differently by the
analyzer and emitter. For example, `List` may mean literal data, a statement sequence,
a function declaration, or a call; `Key(_, Colon, _)` may mean a data pair, a type
annotation, a global form, or a typed record instance. Context determines the meaning,
but the resolution is not persisted.

Consequences visible in the current code include:

- `infer_type` returns the small runtime-oriented `Kind` enum rather than a language
  type and falls back to `Int` or `Symbol` in unresolved cases;
- untyped function parameters currently default to `Int`;
- most user-function parameters are emitted as `i64`, independently of source intent;
- Boolean results are represented as integers;
- record field types remain strings until backend mapping;
- `analyze` reports a narrow class of reassignment errors but returns the original
  `Node`; the normal emitter pipeline does not consume a typed analysis result;
- emitter code still resolves source-level distinctions while generating instructions;
- source-text substring checks currently decide whether host, WASI, or FFI facilities
  are needed.

These are reasonable bootstrap choices, but extending them directly would make every
new feature—optionals, pattern matching, effects, borrowing, traits, lifting—multiply
the number of context-sensitive cases in the backend.

The central invariant should become:

> Backends never infer source meaning. They only lower an already resolved semantic
> program.

## Preserve three distinct representations

### 1. Structural syntax tree: `Node`

Keep `Node` close to source and data. It should preserve:

- source spans and comments through `Meta`;
- bracket and separator choices when useful for formatting;
- unresolved names and overloaded surface operators;
- data literals exactly enough for round-trip serialization;
- alternative accepted spellings.

Do not add ownership, inferred effects, monomorphized types, or backend layouts to
`Node`. Those facts do not belong to a general data format.

### 2. Semantic IR: `Program`

Add a new `src/semantic/` module. A minimal model is:

```rust
pub struct Program {
    pub types: Vec<TypeDecl>,
    pub functions: Vec<FunctionDecl>,
    pub globals: Vec<GlobalDecl>,
    pub entry: ExprId,
    pub expressions: Arena<Expr>,
}

pub struct Expr {
    pub kind: ExprKind,
    pub ty: TypeId,
    pub effects: EffectSet,
    pub span: Span,
}

pub enum ExprKind {
    Literal(Literal),
    Local(LocalId),
    Global(GlobalId),
    Let { binding: LocalId, value: ExprId, body: ExprId },
    Assign { place: Place, value: ExprId },
    Call { callee: FunctionId, arguments: Vec<ExprId> },
    Record { ty: TypeId, fields: Vec<(FieldId, ExprId)> },
    Field { base: ExprId, field: FieldId },
    List(Vec<ExprId>),
    If { condition: ExprId, then_branch: ExprId, else_branch: ExprId },
    Match { scrutinee: ExprId, arms: Vec<MatchArm> },
    Block(Vec<ExprId>),
    Coerce { value: ExprId, coercion: Coercion },
}
```

Use stable IDs rather than names after resolution. Store source spans on every semantic
node, and retain a link to the originating `Node` when diagnostics or source projection
need it.

The initial `Type` model only needs:

```text
Unit, Bool, Int(width), Float(width), Codepoint, Text
List(T), Record(TypeId), Function(params, result, effects)
Option(T), Result(T, E), TypeVar, Error
```

`Symbol` and general `Node` should remain valid explicit data types. They should not be
fallback types for unresolved program identifiers.

### 3. Lowered IR and representation plan

After semantic checking, lower to a smaller backend-facing IR with explicit control
flow and operations. Each value has a `ValueId` and a semantic `TypeId`. A separate
representation plan maps it to forms such as:

```text
I32 | I64 | F32 | F64 | GcRef(type) | NodeRef | LinearMemorySlice
```

This separation is where stack versus GC allocation, boxing, layout, copying, and
specialization belong. Semantic equivalence must not depend on which valid plan is
chosen.

## Minimal semantic core

The first coherent core should be deliberately smaller than the accepted syntax:

1. literals: unit, Boolean, integer, float, codepoint, text;
2. immutable local bindings and lexical blocks;
3. typed reassignment as an explicit effectful operation;
4. first-order functions, calls, and recursion;
5. lists with one element type;
6. nominal records with named fields;
7. `if` expressions with Boolean conditions and unified branch types;
8. algebraic `Option<T>` and `Result<T, E>`;
9. exhaustive `match` over those variants;
10. explicit host functions described by typed signatures and effects.

Traits, closures, generalized lifting, ad-hoc linguistic binding, parallel execution,
and user-defined operators should follow only after this core elaborates deterministically.

## Elaboration and type inference

Elaboration should be a sequence of inspectable passes:

```text
Node
  -> classify syntax by context
  -> resolve declarations and names
  -> generate type/effect constraints
  -> solve and diagnose ambiguity
  -> insert explicit coercions and desugarings
  -> validate exhaustiveness and effects
  -> typed Program
```

Start with local Hindley-Milner-like inference for ordinary expressions, but avoid
claiming full HM while records, effects, overloads, and subtyping are unsettled.
Important rules are more valuable than theoretical breadth:

- numeric literals begin as constrained numeric type variables;
- safe, widening coercions are inserted explicitly in semantic IR;
- narrowing and lossy conversions require source intent;
- branches must unify rather than silently selecting a fallback kind;
- unknown names and unsolved overloads are errors, never `Symbol` or `Int` defaults;
- exported and recursive functions may require annotations when inference is not
  principal or predictable;
- every accepted program serializes its resolved types and chosen overloads.

The existing `Kind` should remain the ABI/runtime tag enum. It should not grow into the
source language's type system.

## Effects

Effects belong in function types and call sites, not in backend feature switches.
Begin with a small closed set:

```text
Pure, State, Allocation, IO, FFI, Async, Unsafe
```

An `EffectSet` is inferred as the union of subexpression effects. Host and FFI
declarations supply trusted effect signatures; callers acquire those effects. Functions
may declare constraints such as `! Pure` or `! IO, Error`, and inference checks the
declaration rather than guessing from source substrings.

`Error` should be modeled first as `Result<T, E>`, not as an invisible control effect.
Unchecked traps and panic-like behavior may later be tracked separately.

This immediately improves the current pipeline: import emission and tree shaking can
be driven by resolved calls and effects rather than scanning source text.

## Ownership and resource inference

Ownership should be a post-type-check analysis over semantic IR, because overload and
control-flow resolution must already be stable. It should not change program meaning.

Start with this deterministic policy:

1. scalars and explicitly copyable values are copied;
2. immutable non-escaping values are borrowed or stack represented;
3. a value with one consuming use may be moved or uniquely allocated;
4. escaping or multiply shared immutable values use a GC reference in the WASM backend;
5. mutation requires a unique place; otherwise use copy-on-write or report that an
   explicit sharing/mutation choice is required;
6. linear resources such as files must be consumed or closed on every path.

Represent the result as facts and constraints, for example:

```rust
pub enum Ownership { Copy, Borrowed(RegionId), Unique, Shared, Linear }
pub enum Storage { Scalar, Stack, WasmGc, LinearMemory, HostHandle }
```

The first implementation does not need a Rust-style lifetime language. Lexical borrows,
escape analysis, last-use moves, and WASM GC fallback cover a useful vertical slice.

Surface forms such as `borrow x`, `move x`, and `copy x` should initially be assertions
on the inferred plan. `@noalloc` is likewise a checked constraint: compilation fails
with an allocation trace if the optimizer cannot satisfy it. These annotations must
not silently request different observable semantics.

## Syntax, IR, and backend boundaries

| Concern | Surface syntax | Semantic IR | Backend plan |
| --- | --- | --- | --- |
| Alternative spellings and Unicode | yes | one canonical operation | no |
| Resolved name/overload | optional or concise | explicit stable ID | no |
| Types and inserted coercions | annotations when useful | always explicit | lowered representation |
| Option/result and pattern match | yes | explicit variants and arms | tags/control flow |
| Effects | optional constraints | inferred explicit set | imports/calls |
| Borrow/move/copy | optional constraint | ownership fact | instruction/layout choice |
| Stack/heap/GC/boxing | no | semantic constraints only | yes |
| SIMD/parallel/no-allocation | optional contract | checked requirement | selected implementation |
| Source style | multiple accepted views | absent | absent |

## Dangerous implicitness

The following should not be added as emitter heuristics:

- guessing unresolved identifiers from nearby names;
- selecting omitted arguments from ambient locals;
- arbitrary truthiness across numbers, text, collections, and user objects;
- transparent `Option`/`Result` unwrapping;
- automatic broadcasting without a type-directed, law-governed lifting rule;
- choosing an overload based on backend representation or optimization success;
- silently inserting lossy numeric conversions;
- allowing effects to appear only after inlining or code generation;
- allowing an agent to reinterpret source on every build.

An agent or IDE may propose a resolution, but acceptance must persist a deterministic
choice in a sidecar or the semantic artifact. Builds consume that choice without an
agent in the loop.

## Minimal vertical slice

The first slice should prove the new boundary without replacing the whole compiler.

### Programs

```warp
square(x) := x * x
square(3)
```

```warp
class Person { name: Text age: Int }
adult(p: Person) := p.age >= 18
adult(Person { name: "Alice" age: 30 })
```

```warp
safe_head(xs: [Int]) -> Int? :=
    if xs.count == 0 then None else Some(xs#0)

match safe_head([1 2 3]) {
    Some(x) => x
    None => 0
}
```

### Implementation sequence

1. Add `src/semantic/{mod,types,ir,diagnostic}.rs` with stable IDs and spans.
2. Add `elaborate(&Node) -> Result<Program, Diagnostics>` for literals, bindings,
   arithmetic, calls, blocks, conditionals, and existing record declarations.
3. Move name and type decisions out of emitter paths for this subset. Preserve the old
   path behind a temporary compatibility entry point.
4. Add a small semantic-IR interpreter for the subset.
5. Add `emit_program(&Program)` and lower the same subset to existing WASM constructors
   and GC types.
6. Differential-test `source -> IR interpreter` against
   `source -> IR -> WASM -> Node`.
7. Add Option/Result variants and exhaustive match.
8. Infer effects from resolved host calls.
9. Add an ownership report, then enforce `borrow`/`move`/`copy`/`@noalloc` constraints.
10. Retire direct source-`Node` interpretation in the emitter feature by feature.

Each migrated construct should have golden tests for:

- parsed `Node` shape;
- elaborated typed IR;
- diagnostic spans for rejected programs;
- interpreter result;
- WASM round-trip result;
- deterministic serialized IR;
- WebAssembly names for modules, functions, types, locals, and fields.

## Near-term repository changes

The safest next changes are structural rather than syntactic:

- introduce `TypeId`, `ExprId`, `FunctionId`, `LocalId`, and `Span`;
- split language `Type` from runtime `Kind`;
- make unresolved inference an error instead of a default;
- define a single AST/IR walker rather than repeating recursion in analysis passes;
- have analysis return a typed artifact plus diagnostics;
- have the WASM emitter accept only that artifact for the migrated subset;
- derive imports, tree shaking, and feature requirements from resolved operations;
- preserve all inferred facts in a deterministic, serializable semantic artifact.

This route keeps the project's existing strengths—the universal data tree, expressive
surface, direct WASM GC, and round-trip discipline—while creating the place where the
central research ideas can be implemented and tested instead of remaining backend
heuristics.
