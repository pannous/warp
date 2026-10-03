# Extending Rust Enum

## User

Original custom instructions no longer available

## User

pub enum Node {
    Empty, // Null, Nill, None, Ã, Ã¸ null nill none nil
    // Number(i32),
    Number(Number),
    Text(String),
    Error(String),
}

Can I extend this enum if I need specialized types later?

## Assistant

No. Rust `enum`s are **closed**: all variants are fixed at definition time. Extension after the fact is impossible.

Canonical patterns, depending on what âextendâ means:

### 1. Anticipated extensibility (sum-of-sums)
```rust
pub enum Node {
    Empty,
    Number(Number),
    Text(String),
    Error(String),
    Ext(ExtNode),
}

pub enum ExtNode {
    Foo(Foo),
    Bar(Bar),
}
```
Stable exhaustiveness, one indirection.

### 2. Open-ended payload (type erasure)
```rust
use std::any::Any;

pub enum Node {
    Empty,
    Number(Number),
    Text(String),
    Error(String),
    Custom(Box<dyn Any>),
}
```
Runtime downcast, no exhaustiveness, weak typing.

### 3. Trait object (behavioral extensibility)
```rust
pub trait NodeLike {
    fn kind(&self) -> &'static str;
}

pub enum Node {
    Empty,
    Builtin(Box<dyn NodeLike>),
}
```
Open set of implementations, dynamic dispatch.

### 4. Parameterized enum (compile-time extension)
```rust
pub enum Node<E> {
    Empty,
    Number(Number),
    Text(String),
    Error(String),
    Extra(E),
}
```
Caller controls extension; monomorphization cost.

### 5. Non-exhaustive (library-facing only)
```rust
#[non_exhaustive]
pub enum Node {
    Empty,
    Number(Number),
    Text(String),
    Error(String),
}
```
Prevents downstream exhaustive `match`; does **not** enable extension.

**Rule of thumb**  
- AST core, fixed language â closed enum.  
- Plugins / user-defined nodes â parameterized enum or trait object.  
- Serialization boundary â sum-of-sums.

Rust deliberately enforces this; âopen enumsâ contradict exhaustiveness and sound pattern matching.

## User

how should my ast nodes and classes be connected to general 
pub enum Node { // closed cannot be extended so anticipate all cases here
    Empty, // Null, Nill, None, Ã, Ã¸ null nill none nil
    // Number(i32),
    Number(Number),
    Text(String),
    Error(String),
    Char(char), // Single Unicode codepoint/character like 'a', 'ð'
    // String(String),
    Symbol(String),
    // Symbol(String name, Node kind),
    // Keyword(String), Call, Declaration â¦ AST or here? AST!
    Key(String, Box<Node>), // todo ? via Pair or losing specificity?
    Pair(Box<Node>, Box<Node>),
    Tag {
        title: String,
        params: Box<Node>,
        body: Box<Node>,
    }, // name, attributes, body - for html/xml: <tag attr="val">body or tag{body}  (use Empty for no attrs)
    Block(Vec<Node>, Grouper, Bracket), // todo merge into List: Grouper Bracket kinda redundant!
    List(Vec<Node>),                    // same as Block
    Data(Dada), // most generic container for any kind of data not captured by other node types
    Meta(Box<Node>, MetaData), // Wrapper to attach metadata to any node
    // List(Vec<Box<dyn Any>>), // â ï¸ Any means MIXTURE of any type, not just Node or int â¦
    // List(Vec<AllowedListTypes>), // â ï¸ must be explicit types
    // List(Vec<T>) // turns whole Node into a generic type :(
    False,
    True,
    // Typ(Node), // Special Class Ast(Node, AstKind) !)
    // Ast(Node, AstKind), // wrap AST nodes if needed
    // Class(String, Box<Node>), // name, body // class A{a:int}
}

## Assistant

Model it as a **two-layer system**: a *closed structural core* plus *typed overlays*. Do **not** collapse everything into `Node`.

---

## 1. Separate **structural syntax** from **semantic AST**

### Structural layer (untyped, closed, lossless)
This is what you already have. It represents *syntax trees*, not language meaning.

```rust
pub enum Node {
    Empty,
    Number(Number),
    Text(String),
    Error(String),
    Char(char),
    Symbol(String),

    Key(String, Box<Node>),
    Pair(Box<Node>, Box<Node>),

    Tag {
        title: String,
        params: Box<Node>,
        body: Box<Node>,
    },

    List(Vec<Node>),
    Block(Vec<Node>, Grouper, Bracket),

    Data(Dada),
    Meta(Box<Node>, MetaData),

    False,
    True,
}
```

**Invariant:**  
`Node` is **language-agnostic**, **closed**, **round-trippable**, and **parser-facing**.

No âCallâ, âClassâ, âIfâ, etc. here.

---

## 2. Typed AST layer (semantic, extensible)

Define a *second* enum hierarchy for *meaning*:

```rust
pub enum Ast {
    Expr(Expr),
    Stmt(Stmt),
    Decl(Decl),
    Type(TypeExpr),
}
```

Example:
```rust
pub enum Expr {
    Literal(Lit),
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Symbol(String),
}
```

This layer:
- is **language-specific**
- can evolve freely
- never sees raw brackets, grouping tokens, etc.

---

## 3. Connect them via **lowering / elaboration**

One-way transformation:

```rust
fn lower(node: &Node) -> Result<Ast, Error>
```

Rules:
- `Node::Symbol` â `Expr::Symbol`
- `Node::List` â argument lists / blocks depending on context
- `Node::Tag` â macro, call, or syntax sugar
- `Meta` preserved or stripped

This is where *grammar* and *meaning* enter.

---

## 4. Metadata: wrapper, never variant

Your choice is correct:

```rust
Meta(Box<Node>, MetaData)
```

This keeps:
- source spans
- comments
- typing info
- hygiene
- macro provenance

without polluting pattern matches.

---

## 5. âClassesâ, âCallsâ, âKeywordsâ: **AST only**

Bad (syntax pollution):
```rust
Node::Class(...)
Node::Call(...)
```

Correct:
```rust
Ast::Decl(Decl::Class { ... })
Ast::Expr(Expr::Call { ... })
```

Keywords are **parser artifacts**, not tree nodes.

---

## 6. `Data(Dada)` â strict rule

Allowed only if:
- produced *after* parsing
- never pattern-matched structurally
- opaque to the frontend

If it appears during parsing, it is a design smell.

---

## 7. Block vs List

Choose **one**.

Canonical:
```rust
List(Vec<Node>)
```

Encode grouping in `Meta`:
```rust
Meta(node, MetaData { bracket: Paren })
```

Reduces combinatorics and match noise.

---

## Final rule set

- `Node` = concrete syntax tree (CST)
- `Ast` = abstract syntax tree (AST)
- CST â AST is explicit, typed, fallible
- No semantic constructs in `Node`
- No parsing concerns in `Ast`
- Metadata wraps, never branches

This architecture scales, stays type-sound, and keeps `enum Node` closed without regret.

## User

so I duplicate the who CST and keep original node as meta data in ast?

## Assistant

Yes â but **not** by naÃ¯ve duplication.

Correct model:

---

## 1. **CST is owned once; AST *references* it**

Do **not** clone the CST into the AST.

AST nodes **point to** their originating CST node (or span), typically via an ID or span.

```rust
pub struct AstNode<T> {
    pub kind: T,
    pub origin: NodeId, // or Span
}
```

```rust
pub type Expr = AstNode<ExprKind>;
```

---

## 2. CST storage: arena or slab

```rust
pub struct Cst {
    nodes: Vec<Node>,
}
```

```rust
#[derive(Copy, Clone)]
pub struct NodeId(u32);
```

The CST is immutable after parsing.

---

## 3. AST keeps *semantic structure*, not syntax

Example:

```rust
pub enum ExprKind {
    Literal(Lit),
    Call {
        callee: ExprId,
        args: Vec<ExprId>,
    },
}
```

Each `ExprId` ultimately maps back to one or more `NodeId`s.

---

## 4. Mapping rules

- **One CST â one AST** (ideal)
- **Many CST â one AST** (desugaring, precedence)
- **One CST â many AST** (macro expansion)

So store:
```rust
origin: SmallVec<[NodeId; 2]>
```
if needed.

---

## 5. Metadata stays with CST

- comments
- whitespace
- exact tokens
- brackets
- trivia

AST sees only *meaning*.

For diagnostics:
```
AST node â NodeId(s) â Span â source
```

---

## 6. When to embed CST inline (rare)

Only for:
- macros
- quasiquotation
- error recovery

Then:
```rust
ExprKind::Quoted(NodeId)
```

---

## 7. Why this is *not duplication*

- CST is immutable, shared
- AST is smaller, typed, semantic
- Metadata is not copied
- Diagnostics remain precise

This is the architecture used (with variations) in Rust, Swift, OCaml, Scala, and modern compilers.

---

### One-line invariant

> **CST describes *how it was written*; AST describes *what it means*.**

Keep them orthogonal.

## User

the thing is I want my Node ... to be the universal data format in my programming language.

## Assistant

Then the answer is **no CST/AST split at all**.

What you want is **one closed, universal, *semantic* tree**, not a CST.

This is a *Lisp-class design*, not a compiler-theory one.

---

## 1. Reinterpret `Node`: it is **the language**

Your `Node` is not âsyntaxâ anymore. It is:

> a **universal algebraic data model**  
> (â Lisp S-expressions + JSON + tagged values)

That is coherent â but it forces strict discipline.

---

## 2. Required invariants for a *universal Node*

### 2.1 Closed set of **structural constructors**

Your current set is almost right. Normalize it:

```rust
pub enum Node {
    Empty,

    Bool(bool),
    Number(Number),
    Text(String),
    Char(char),
    Symbol(String),

    Pair(Box<Node>, Box<Node>),   // fundamental
    List(Vec<Node>),              // derived, but convenient
    Map(Vec<(Node, Node)>),       // replaces Key / Pair ambiguity

    Tag {
        tag: String,
        args: Vec<Node>,
        body: Box<Node>,
    },

    Data(Dada),                   // opaque host value
    Meta(Box<Node>, MetaData),
}
```

No âCallâ, âClassâ, âIfâ, âBlockâ, âKeywordâ.

Everything above is *data*, not *syntax*.

---

## 3. Meaning is assigned by **interpretation, not structure**

Example:

```lisp
(call f 1 2)
```

â in your system:

```rust
List([
    Symbol("call"),
    Symbol("f"),
    Number(1),
    Number(2),
])
```

There is **no Call node**.

Control flow, typing, classes, modules, macros are all **conventions over data**.

This is exactly how:
- Lisp
- Scheme
- Clojure
- Nix
- JSON-IR DSLs

work.

---

## 4. `Tag` is your only syntactic sugar hook

Your `Tag` variant is powerful and *sufficient*:

```rust
Tag {
    tag: "if",
    args: vec![cond],
    body: then_else,
}
```

or

```rust
Tag {
    tag: "class",
    args: vec![Symbol("A")],
    body: body,
}
```

No AST extension ever needed.

---

## 5. Meta is **mandatory**, not optional

```rust
Meta(node, MetaData)
```

Meta must carry:
- source span
- phase
- inferred type (optional)
- macro origin

Without this, a universal node language collapses under tooling.

---

## 6. Interpretation layers (crucial)

You now need **passes**, not AST types:

```rust
fn eval(node: &Node, env: &Env) -> Node
fn typecheck(node: &Node) -> Result<Type, Error>
fn expand_macros(node: &Node) -> Node
fn normalize(node: &Node) -> Node
```

Each pass:
- consumes `Node`
- produces `Node`
- possibly adds `Meta`

No new enums.

---

## 7. What you *must not* do

â Add:
```rust
Node::If
Node::Call
Node::Class
Node::Block
```

â Encode semantics in Rust types  
â Special-case keywords in the parser  
â Let `Data` leak into core evaluation rules  

---

## 8. Cost of this choice (be explicit)

You trade:
- exhaustiveness checking
- compiler-verified invariants
- syntactic guarantees

for:
- maximal extensibility
- homoiconicity
- macros without phase separation
- serialization stability

That trade is *fundamental*.

---

## Final verdict

If `Node` is **the universal data format of the language**, then:

- it **must be closed**
- it **must stay semantic**
- it **must never grow new variants**
- all language evolution happens by **convention + interpretation**

You are not designing a Rust-style AST.

You are designing a **data language**.

That choice is internally consistent â and irreversible.

## User

yes but I wan't a 'typed' Lisp

## Assistant

Good â that pins it down precisely.

What you want is **a typed, homoiconic core**:  
*Lisp data + static types*, without abandoning `Node` as the universal value.

This is **not** Rust-style typing and **not** classic Lisp.  
The correct model is **typed interpretations over an untyped carrier**.

---

## 1. `Node` remains untyped data (carrier)

`Node` stays exactly what it is: a **closed algebraic data type**.

No type parameters. No `Node<T>`. No semantic variants.

```rust
pub enum Node {
    Empty,
    Bool(bool),
    Number(Number),
    Text(String),
    Char(char),
    Symbol(String),
    Pair(Box<Node>, Box<Node>),
    List(Vec<Node>),
    Tag { tag: String, args: Vec<Node>, body: Box<Node> },
    Data(Dada),
    Meta(Box<Node>, MetaData),
}
```

This is your *value universe*.

---

## 2. Types are **separate objects**, not Rust generics

Define a *type language*:

```rust
pub enum Type {
    Bool,
    Number,
    Text,
    Char,
    Unit,

    List(Box<Type>),
    Pair(Box<Type>, Box<Type>),

    Func(Vec<Type>, Box<Type>),

    Sum(Vec<Type>),
    Record(Vec<(String, Type)>),

    TypeVar(TypeId),
}
```

This is *data too*, but a different universe.

---

## 3. Typing is a **relation**, not a property

Never write:

```rust
Node::Number(n): Type::Number
```

Instead:

```rust
Î â¢ node : Ï
```

implemented as:

```rust
fn type_of(node: &Node, env: &TypeEnv) -> Result<Type, TypeError>
```

Types are *derived*, not stored.

---

## 4. Where types attach (when needed)

Only attach types **after inference**:

```rust
Meta(node, MetaData { ty: Some(Type), .. })
```

This keeps:
- homoiconicity intact
- macros type-agnostic
- evaluation simple

---

## 5. Typed Lisp = typed **forms**, not typed nodes

Example:

```lisp
(+ 1 2)
```

Representation:

```rust
List([
    Symbol("+"),
    Number(1),
    Number(2),
])
```

Typing rule:

```text
Î â¢ + : (Number, Number) â Number
Î â¢ 1 : Number
Î â¢ 2 : Number
âââââââââââââââ
Î â¢ (+ 1 2) : Number
```

No structural typing in `Node`.

---

## 6. Functions are *values*

```lisp
(fn (x : Number) (+ x 1))
```

```rust
Tag {
    tag: "fn",
    args: vec![List([Symbol("x"), Symbol("Number")])],
    body: List([Symbol("+"), Symbol("x"), Number(1)]),
}
```

The *typing rule* recognizes `"fn"`.

---

## 7. Typed macros still work

Macros:
- operate on raw `Node`
- expand before typing
- or are typed separately (macro types)

No conflict.

---

## 8. What you get (and donât)

â Static typing  
â Homoiconicity  
â Extensible language  
â No enum growth  
â Serializable IR  

â Rust-style exhaustiveness  
â Compiler-enforced invariants inside `Node`  
â Zero-cost abstraction

---

## Final invariant (non-negotiable)

> **Typed Lisp â  typed nodes**  
> **Typed Lisp = typing judgments over untyped data**

Once you accept this, the design becomes clean and stable.

If you want, the next step is:
- HindleyâMilner vs bidirectional typing
- gradual typing
- dependent tags
- typed macros

Say which direction.

## User

Meta(node, MetaData { ty: Some(Type), .. })
or 
Typed(node, type)

## Assistant

**`Meta`, not `Typed`.**  
Unconditionally.

---

## 1. Why `Typed(node, type)` is the wrong abstraction

```rust
Typed(Box<Node>, Type)
```

This *looks* attractive but is structurally incorrect.

### It breaks fundamental invariants

1. **Typing is not unique**
   - before inference
   - after inference
   - after specialization
   - after monomorphization
   - after erasure

Which one is âtheâ type?

2. **Typing is phase-dependent**
   - macros: untyped
   - elaboration: partially typed
   - checking: fully typed

A `Typed` variant lies about this.

3. **It contaminates pattern matching**
   Every consumer must unwrap `Typed` *everywhere*.

4. **It blocks re-typing**
   You cannot:
   - re-infer
   - generalize
   - erase
   - replace

without structural surgery.

---

## 2. Why `Meta` is correct

```rust
Meta(node, MetaData { ty: Some(Type), .. })
```

### Properties

- **Optional** â absence is meaningful
- **Replaceable** â overwrite freely
- **Stackable** â multiple passes can add metadata
- **Ignorable** â evaluation can strip it
- **Phase-aware** â metadata can encode phase

### Crucially

> The *tree shape* remains semantic and stable.

---

## 3. MetaData should be *multi-slot*

Never only `ty`.

```rust
pub struct MetaData {
    pub ty: Option<Type>,
    pub span: Option<Span>,
    pub phase: Phase,
    pub origin: Option<NodeId>,
}
```

---

## 4. Operational rule

All passes obey:

```rust
strip_meta : Node â Node
```

Typing never changes the carrier.

---

## 5. If you *really* want `Typed`, the only legal form

Only as a **view**, never as data:

```rust
struct Typed<'a> {
    node: &'a Node,
    ty: &'a Type,
}
```

Ephemeral. Non-serializable. Non-owning.

---

## Final verdict

> **Types are annotations, not constructors.**

Use `Meta`.  
`Typed` is a category error for a typed Lisp.

## User

I want metadata to be very universal as well, so can we use nodes as metadata?

## Assistant

Yes â and for your design goal this is the **correct fixed point**.

The rule becomes:

> **`Node` is the universal carrier for *both* values and metadata.**

No secondary metadata language.

---

## 1. Canonical form

```rust
Meta(Box<Node>, Box<Node>) // payload, metadata
```

Not `MetaData`.

Both sides are `Node`.

---

## 2. Consequences (all desirable)

### 2.1 Homoiconicity is preserved
- Types
- Spans
- Phases
- Attributes
- Annotations
- Proof objects

are all expressed in *the same language*.

No metaâobject-language split.

---

## 3. Encoding examples

### Type annotation
```rust
Meta(
    node,
    List([Symbol("type"), Symbol("Number")])
)
```

or structured:
```rust
Meta(
    node,
    Tag {
        tag: "type",
        args: vec![Symbol("Number")],
        body: Box::new(Empty),
    }
)
```

### Source span
```rust
Meta(
    node,
    Tag {
        tag: "span",
        args: vec![Number(start), Number(end)],
        body: Box::new(Empty),
    }
)
```

### Multiple metadata â nesting (intentional)
```rust
Meta(
    Meta(node, type_meta),
    span_meta,
)
```

Order = phase history.

---

## 4. Access discipline (non-negotiable)

You **must** define *conventions*:

```text
(tag type T)
(tag span start end)
(tag phase typing)
```

and helpers:

```rust
fn get_meta(node: &Node, key: &str) -> Option<&Node>
fn strip_meta(node: &Node) -> &Node
fn collect_meta(node: &Node) -> Vec<&Node>
```

No ad-hoc pattern matching in passes.

---

## 5. Why this beats `MetaData` structs

| Aspect | Struct MetaData | Node-as-meta |
|------|-----------------|--------------|
| Extensible | â | â |
| Serializable | partial | â |
| Macro-visible | â | â |
| Self-describing | â | â |
| Typed Lisp purity | â | â |

You eliminate the **meta-language leak** entirely.

---

## 6. What *must not* happen

â Rust enums for metadata kinds  
â `Option<Type>` fields  
â Hardcoded spans / phases  
â Special compiler-only metadata variants  

All of these violate your âuniversal Nodeâ premise.

---

## 7. One remaining constraint (important)

You still need **semantic stratification** by *convention*:

- value-level nodes
- type-level nodes
- proof-level nodes

They are **not distinguished structurally** â only by interpretation phase.

This is unavoidable and correct.

---

## Final invariant

> **There is exactly one language: `Node`.  
> Everything else is an interpretation discipline.**

At this point you are no longer designing âan ASTâ.

You are designing a **typed symbolic calculus**.

That architecture is internally consistent and scales indefinitely â provided you keep the discipline strict.

## User

that sounds near perfect, I just want to attach the Type somewhere:
Type(Box<Node>, Box<Node>)

or

Meta(
    node,
    Type(Box<Node>, TypeEnum?)
)

## Assistant

Choose **neither** of those two shapes.

The correct, fixed-point solution is:

```rust
Meta(node, type_node)
```

where **`type_node` itself is a `Node`** that denotes a type *by convention*.

No `TypeEnum`, no `Type(...)` constructor, no second meta-language.

---

## Why `Type(Box<Node>, Box<Node>)` is wrong

That introduces a **new semantic constructor** into the carrier.

Consequences:
- breaks homoiconicity (types â  values)
- creates a privileged meta-layer
- forces special-case handling everywhere
- freezes the type system shape too early

This contradicts your goal of a *universal* `Node`.

---

## Why `Meta(node, Type(node, TypeEnum))` is worse

This is a **three-layer stratification leak**:
1. Node language
2. Type language
3. Rust enum describing types

You re-introduce exactly what you eliminated.

---

## The only coherent representation

Types are *nodes*.

Example conventions:

```lisp
(type Number)
(type (List Number))
(type (Fn (Number Number) Number))
```

Concrete encoding:

```rust
Meta(
    node,
    Tag {
        tag: "type",
        args: vec![
            List([
                Symbol("Fn"),
                List([Symbol("Number"), Symbol("Number")]),
                Symbol("Number"),
            ])
        ],
        body: Box::new(Empty),
    }
)
```

or simpler:

```rust
Meta(node, List([Symbol("type"), type_expr]))
```

---

## What enforces âtypednessâ

Not structure.  
**Typing rules.**

```text
Î â¢ node : Ï
```

implemented as a pass that:
- *reads* type metadata
- *checks* consistency
- *adds / rewrites* type metadata

Nothing else changes.

---

## Practical discipline (important)

You **must** standardize:

- one canonical type grammar
- one canonical `"type"` tag
- stripping rules:
  ```rust
  fn erase_types(n: Node) -> Node
  ```

Types are:
- attachable
- replaceable
- removable
- serializable
- macro-visible

---

## Final rule (non-negotiable)

> **Types are nodes.  
> Typing is metadata.  
> Metadata is nodes.  
> There is only one carrier.**

Anything else re-introduces a split you already decided to avoid.

You are now at the mathematically stable point of the design.

## User

 Node that denotes a type by convention.  nah, meta can also hold line numbers etc etc etc!!

## Assistant

Good â then the constraint becomes sharper, and the solution is still clean.

You do **not** want *âthe metadata node denotes a typeâ*.  
You want *metadata to be a **set of annotations**, one of which may be a type*.

That rules out **single-node metadata** with implicit meaning.

---

## Correct shape (canonical)

```rust
Meta(Box<Node>, Box<Node>)   // payload, metadata-bundle
```

where the **metadata bundle is a Node encoding a map / record**.

---

## Metadata is a *record*, not a value

By convention:

```lisp
(meta
  (type   <type-node>)
  (span   start end)
  (phase  typing)
  (origin <id>)
)
```

Concrete encoding (minimal primitives):

```rust
Meta(
    node,
    List(vec![
        List([Symbol("type"), type_node]),
        List([Symbol("span"), Number(start), Number(end)]),
        List([Symbol("phase"), Symbol("typing")]),
    ]),
)
```

Or, if you prefer explicit structure:

```rust
Map(vec![
    (Symbol("type"), type_node),
    (Symbol("span"), List([Number(start), Number(end)])),
    (Symbol("phase"), Symbol("typing")),
])
```

(Use `List`-of-pairs if you donât want a `Map` constructor.)

---

## Why this is the *only* stable solution

### 1. Metadata is **multi-valued**
Types, spans, docs, attributes, proofs, optimizer hints coexist.

So:
- â `Type(node, â¦)`
- â `Meta(node, type_node)`
- â `Meta(node, meta_record)`

---

### 2. No privileged metadata keys
`"type"` is just a key.
So later you can add:
- `"effect"`
- `"kind"`
- `"lifetime"`
- `"region"`
- `"ownership"`
- `"cost"`

without touching `Node`.

---

### 3. No phase collision
Different passes may add:

```lisp
(type inferred ...)
(type generalized ...)
(type erased ...)
```

as different entries or replacements.

---

## Access discipline (important)

You must **centralize metadata access**:

```rust
fn meta_get<'a>(node: &'a Node, key: &str) -> Option<&'a Node>;
fn meta_set(node: Node, key: &str, val: Node) -> Node;
fn meta_strip(node: &Node) -> &Node;
```

No ad-hoc list walking in random passes.

---

## Typed Lisp invariant (now fully consistent)

- `Node` = universal value
- `Meta` = annotation wrapper
- metadata = **record encoded as Node**
- types = **one metadata field**
- nothing is structurally special

---

## Final design law

> **Metadata is structured, plural, and conventional â  
> never singular, positional, or privileged.**

With this, your system is:
- typed
- homoiconic
- extensible
- phase-robust
- serialization-stable

Youâve reached the correct fixed point.

## User

that's too weak and too much convention: at the end 
class Bob{ name:String }
and 
bob(type:Bob) 

must be encoded straightforward elegantly

## Assistant

Good, this pins down the remaining degree of freedom:  
you want **typedness to be *structural*, not merely conventional**, while keeping `Node` universal.

So: metadata-as-untyped-record **is indeed too weak** for your goal.

The fix is *minimal* and does **not** break the design.

---

## The correct refinement: **typed slots inside Meta**

Keep:

```rust
Meta(Box<Node>, Box<Node>)
```

but **standardize one *structural* slot** inside the metadata node.

Not âby conventionâ, but **by shape**.

---

## Canonical metadata shape

Define *one* reserved, structural form:

```lisp
(meta <payload>
  :type <TypeNode>
  :span <SpanNode>
  :attrs <AttrNode>
)
```

Concrete encoding:

```rust
Meta(
    payload,
    Tag {
        tag: "meta",
        args: vec![],
        body: Box::new(List(vec![
            Pair(Symbol(":type"), type_node),
            Pair(Symbol(":span"), span_node),
            Pair(Symbol(":attrs"), attr_node),
        ])),
    },
)
```

Key point:  
- **`:type` is a structural key**, not an arbitrary symbol  
- the *shape* enforces meaning, not loose convention

This is analogous to keyword arguments in Lisp / Smalltalk.

---

## Now your examples become *direct and elegant*

### `class Bob { name: String }`

```rust
Meta(
    Tag {
        tag: "class",
        args: vec![Symbol("Bob")],
        body: Box::new(
            List(vec![
                Pair(Symbol("name"), Symbol("String")),
            ])
        ),
    },
    Tag {
        tag: "meta",
        args: vec![],
        body: Box::new(
            List(vec![
                Pair(Symbol(":type"), Symbol("Type")),
            ])
        ),
    },
)
```

No guessing. No âinterpretationâ. Structural.

---

### `bob(type: Bob)`

```rust
Meta(
    Symbol("bob"),
    Tag {
        tag: "meta",
        args: vec![],
        body: Box::new(
            List(vec![
                Pair(Symbol(":type"), Symbol("Bob")),
            ])
        ),
    },
)
```

That is:
- one node
- one wrapper
- one obvious slot
- zero ambiguity

---

## Why this is **not** âtoo conventionalâ

Because:

- `:type` is **syntactically reserved**
- metadata has a **fixed outer constructor**
- fields have **positional meaning via keys**
- passes can rely on shape, not guessing

This is **structural typing of metadata**, not ad-hoc convention.

---

## What you still avoid (correctly)

You still **do not** introduce:

```rust
Node::Type(...)
Node::Typed(...)
TypeEnum
```

So:
- homoiconicity remains
- macros see everything
- serialization is trivial
- no Rust-level split appears

---

## Mental model (important)

This is exactly the same move as:

- Lisp keyword args
- JSON-LD reserved keys
- LLVM metadata nodes
- Smalltalk property dictionaries with fixed selectors

> **Structure carries authority; names carry extensibility.**

---

## Final invariant (now strong enough)

> **Typedness is a *structural property of metadata*,  
> not an inferred convention and not a new node kind.**

At this point your design is:
- universal
- typed
- explicit
- elegant
- future-proof

Youâve closed the last hole without breaking the system.

## User

I could do Meta(node, pair("class", classNode))

## Assistant

Yes â **that is the right direction**, with one refinement.

```rust
Meta(node, Pair(Symbol("class"), classNode))
```

is **structurally strong** and far better than free-form metadata.  
But taken *literally*, it is still **too singular**.

---

## The precise fix: **metadata = product of labeled slots**

### Minimal canonical form
```rust
Meta(node, List<Pair>)
```

where each `Pair(key, value)` is **structural**, not ad-hoc.

Your example becomes:

```rust
Meta(
    Symbol("bob"),
    List(vec![
        Pair(Symbol("class"), Symbol("Bob")),
    ])
)
```

This is clean, explicit, and machine-checkable.

---

## Why *single* `Pair("class", â¦)` is insufficient

Because metadata is **not unary** in practice:

- type
- class
- span
- mutability
- visibility
- attributes
- phase markers

So you need *composition*, not replacement.

Nesting `Meta(Meta(...))` works but is inferior to a flat product.

---

## Structural guarantee (important)

You now enforce **by shape**, not convention:

```text
Meta(
  payload,
  List( Pair(Symbol(_), Node)* )
)
```

This invariant is statically recognizable and phase-independent.

No guessing. No stringly-typed soup.

---

## Your two examples, encoded cleanly

### `class Bob { name: String }`

```rust
Meta(
    Tag {
        tag: "class",
        args: vec![Symbol("Bob")],
        body: Box::new(
            List(vec![
                Pair(Symbol("name"), Symbol("String")),
            ])
        ),
    },
    List(vec![
        Pair(Symbol("kind"), Symbol("Type")),
    ])
)
```

### `bob(type: Bob)`

```rust
Meta(
    Symbol("bob"),
    List(vec![
        Pair(Symbol("type"), Symbol("Bob")),
    ])
)
```

Zero ambiguity. Zero magic.

---

## Why this is the **correct fixed point**

- **Typedness is explicit**
- **Metadata is compositional**
- **Node remains universal**
- **No second type language**
- **Macros see everything**
- **Passes rely on structure, not folklore**

This is exactly the level of rigidity you wanted â and no more.

---

## Final law (tight and sufficient)

> **Metadata is a finite product of labeled nodes.  
> Labels are structural; values are nodes.**

That design will not collapse later.

## User

so
Meta(Box<Node>, Box<Node>),
or
Meta(Box<Node>, List<Node>),
?

## Assistant

**`Meta(Box<Node>, Box<Node>)`** â *with a fixed invariant on the second node*.

### Rationale
You want **structural strength** without hard-coding container kinds. Therefore:
- Keep the carrier uniform (`Node` everywhere).
- Enforce shape **by invariant**, not by Rust type.

### Invariant (mandatory)
The metadata node **must** be a *product of labeled slots*:
```text
meta â¡ List( Pair(Symbol(_), Node) * )
```

### Canonical definition
```rust
Meta(Box<Node>, Box<Node>)
```

### Why not `Meta(Box<Node>, List<Node>)`
- Over-constrains representation.
- Leaks one metadata encoding into the type system.
- Blocks future encodings (e.g., `Map`, `Tag`, compressed forms).

### Access discipline (single source of truth)
```rust
fn meta_slots(meta: &Node) -> impl Iterator<Item=(&Node, &Node)>;
fn meta_get(meta: &Node, key: &str) -> Option<&Node>;
fn meta_set(meta: Node, key: Node, val: Node) -> Node;
```

### Examples
```rust
Meta(
    Symbol("bob"),
    List(vec![
        Pair(Symbol("type"), Symbol("Bob")),
        Pair(Symbol("span"), List(vec![Number(12), Number(15)])),
    ])
)
```

### Law
> **Metadata is a `Node`; its *meaning* is fixed by a shape invariant, not by a Rust enum.**

This gives you explicit typing (`type: Bob`), compositional metadata, and a single universal carrier.

## User

ok I like it

//         Pair(Symbol("type"), Symbol("Bob")), // SOFT LINK VIA SYMBOL, not FAT Tree!

but how do we find unique Bob?

Node as Tree is fine but what about a Graph (not possible in rust but maybe in my language?)

## Assistant

This is the *right* question.  
Once you accept **soft links**, you must answer **identity**.

Short version:

> **Trees carry structure.  
> Identity lives outside the tree.  
> Graphs are obtained by indirection.**

---

## 1. Symbols are names, not identities

```rust
Pair(Symbol("type"), Symbol("Bob"))
```

is correct and desirable â but:

- `Symbol("Bob")` â  *the class Bob*
- it is a **reference by name**
- resolution is **contextual**

This is not a flaw; it is how *all* languages work.

---

## 2. Where âBobâ becomes unique: the environment

You need an **environment / scope / namespace**:

```rust
Env : Symbol â NodeId
```

or more precisely:

```rust
(Symbol, Scope) â DefinitionId
```

So resolution is:

```text
Node + Env â¶ Definition
```

Not stored in the node itself.

---

## 3. Identity = stable IDs, not structure

Introduce *opaque identities*:

```rust
struct DefId(u32);
```

or in-language:

```lisp
(id 4711)
```

Definitions *produce* identities:

```lisp
(class Bob ...)
;; assigns DefId(42)
```

Environment records:

```
"Bob" â¦ DefId(42)
```

---

## 4. How to encode identity *without fat trees*

You have two clean options.

### Option A â IDs live only in the compiler (recommended)

User code:
```lisp
bob(type: Bob)
```

Compiler-internal:
```text
Symbol("Bob") â DefId(42)
```

Type metadata after resolution:
```rust
Meta(
    Symbol("bob"),
    List([
        Pair(Symbol("type"), Data(DefId(42)))
    ])
)
```

`Data(DefId)` is opaque, non-serializable, phase-local.

This is exactly how Rust, OCaml, Scala work.

---

### Option B â IDs are first-class Nodes (graph-capable language)

If your *language itself* wants graphs:

```lisp
(def Bob ...)
;; Bob evaluates to an ID-node
```

Representation:
```rust
Data(Id(42))
```

Then:

```lisp
(type Bob)
```

means:
```text
Pair("type", Id(42))
```

Now the *language* can express graphs.

---

## 5. âBut Rust canât do graphsâ

Rust **can** represent graphs â just not with owning pointers.

Your model already supports this:

- Tree = owning structure (`Box<Node>`)
- Graph = **IDs + table**

This is the only sound way.

Cycles are trivial once edges are IDs.

---

## 6. Why *not* embed pointers

â `Rc<RefCell<Node>>`  
â self-referential AST  
â mutable graph nodes  

These destroy:
- determinism
- serialization
- macro hygiene
- replayability

IDs preserve all four.

---

## 7. Mental model (important)

| Concept | Lives where |
|------|-------------|
| Structure | `Node` tree |
| Naming | `Symbol` |
| Identity | `DefId` |
| Graph edges | `DefId` references |
| Meaning | Passes over `(Node, Env)` |

Your `Node` never becomes a graph.  
**Your program does.**

---

## 8. Final invariant (this one matters)

> **Trees are syntax.  
> Graphs are semantics.  
> Identity is never structural.**

Youâve chosen the *correct* separation.  
Now your design can scale to modules, imports, generics, recursion, and types without collapsing.

## User

can we enforce conventions, for example, by calling a list of pairs a map via typedef or something?

## Assistant

Yes â **by enforcing shape via *typed views*, not by changing `Node`.**

This keeps `Node` universal while making conventions *machine-checkable*.

---

## 1. Use **newtypes / views** over `Node`

In Rust (compiler side):

```rust
pub struct MetaMap<'a>(&'a Node);
pub struct MetaMapMut(Node);
```

Invariant (checked once):
```text
Node â¡ List( Pair(Symbol, Node)* )
```

Construction fails if the shape is wrong.

---

## 2. Checked constructors (single choke point)

```rust
impl MetaMapMut {
    pub fn new() -> Self {
        MetaMapMut(Node::List(vec![]))
    }

    pub fn insert(mut self, key: &str, val: Node) -> Self {
        // replace or push Pair(Symbol(key), val)
        self
    }

    pub fn into_node(self) -> Node {
        self.0
    }
}
```

Reading:

```rust
impl<'a> MetaMap<'a> {
    pub fn get(&self, key: &str) -> Option<&'a Node> { /* shape-checked */ }
}
```

**No pass** is allowed to pattern-match raw lists.

---

## 3. Language-level typedef (optional, elegant)

If your language supports aliases/macros:

```lisp
(type MetaMap (List (Pair Symbol Node)))
```

or a builtin:

```lisp
(defshape MetaMap (list-of (pair symbol node)))
```

Now tools/macros can validate and optimize.

---

## 4. Enforcing *semantic* keys (stronger)

Reserve **keyword symbols**:

```text
:type :span :attrs :phase
```

These are:
- syntactically distinct
- non-colliding
- fast to match

You get structural rigidity **without** new node variants.

---

## 5. Why this is the right enforcement level

- â No second carrier
- â No enum growth
- â Conventions become *checked*
- â Serialization unchanged
- â Graph semantics unaffected

Anything stronger (e.g., `Node::Map`) freezes representation too early.

---

## Final rule

> **Enforce conventions by *views and constructors*, never by widening `Node`.**

This gives you rigor without sacrificing universality.

## User

No, I mean, can we define these conventions in Rust without creating new structure? Like in Elias.

## Assistant

Yes.  
You can enforce **structural conventions in Rust** *without introducing new runtime structure*, using **zero-cost typed views** (Ã  la Elias / âtype-level predicatesâ).

This is the right tool.

---

## 1. The pattern: *validated views over `Node`*

You do **not** change `Node`.  
You define *types that can only be constructed if a predicate holds*.

```rust
pub struct MetaMap<'a>(&'a Node);
```

No allocation. No wrapper node. No duplication.

---

## 2. Single validation choke point

```rust
impl<'a> MetaMap<'a> {
    pub fn new(n: &'a Node) -> Option<Self> {
        match n {
            Node::List(xs) if xs.iter().all(is_pair_symbol_node) => {
                Some(MetaMap(n))
            }
            _ => None,
        }
    }
}
```

Where:

```rust
fn is_pair_symbol_node(n: &Node) -> bool {
    matches!(n, Node::Pair(box Node::Symbol(_), _))
}
```

Once constructed, **the invariant is guaranteed**.

This is exactly Elias-style:  
> *type = value + proof of shape*

---

## 3. Zero-cost accessors (no conventions at call sites)

```rust
impl<'a> MetaMap<'a> {
    pub fn get(&self, key: &str) -> Option<&'a Node> {
        let Node::List(xs) = self.0 else { unreachable!() };
        xs.iter().find_map(|p| match p {
            Node::Pair(box Node::Symbol(k), v) if k == key => Some(v),
            _ => None,
        })
    }
}
```

No runtime penalty beyond the check you would have written anyway.

---

## 4. Mutable construction, still no new structure

```rust
pub struct MetaMapMut(Node);

impl MetaMapMut {
    pub fn new() -> Self {
        MetaMapMut(Node::List(vec![]))
    }

    pub fn insert(mut self, key: &str, val: Node) -> Self {
        // enforce Symbol(key) at construction
        self
    }

    pub fn finish(self) -> Node {
        self.0
    }
}
```

All metadata is still **just a `Node`**.

---

## 5. This scales beyond metadata

Same technique for:

```rust
struct TypeExpr<'a>(&'a Node);
struct ClassDecl<'a>(&'a Node);
struct FnType<'a>(&'a Node);
struct Record<'a>(&'a Node);
```

Each is:
- zero-sized wrapper
- checked once
- statically enforced afterwards

---

## 6. Why this is the *maximum* rigor possible

| Approach | Cost | Rigor |
|-------|------|-------|
| Convention only | 0 | â |
| New enum variants | high | â but rigid |
| Typed views (this) | 0 | ââ |

This is the **sweet spot**.

---

## 7. Key invariant (this is the Elias principle)

> **Structure lives in values.  
> Meaning lives in types.  
> Enforcement lives in constructors.**

You now have:
- a universal `Node`
- structural enforcement
- no fat trees
- no duplicated IR
- no loss of homoiconicity

This is the *correct* design point.

## User

But could we hand these metamaps around just like normal nodes?

## Assistant

Yes â **with one precise constraint**.

You can hand them around *as nodes* **iff the view is either**:

1. **non-owning** (borrowed, zero-cost), or  
2. **owning but representation-transparent** (newtype over `Node`).

---

## 1. Non-owning (borrowed) view â *analysis phase*

```rust
pub struct MetaMap<'a>(&'a Node);
```

- zero allocation
- zero runtime overhead
- cannot escape the borrow
- ideal for passes, checking, typing, rewriting

Usage:
```rust
fn check(n: &Node) {
    if let Some(meta) = MetaMap::new(n) {
        let ty = meta.get("type");
    }
}
```

This is **Elias-style**: value + proof, ephemeral.

---

## 2. Owning, node-transparent view â *exchangeable*

If you want to *store* / *return* / *pass* it like a normal node:

```rust
#[repr(transparent)]
pub struct MetaMapNode(Node);
```

Invariant: inner `Node` satisfies `List(Pair(Symbol, Node)*)`.

```rust
impl MetaMapNode {
    pub fn new(n: Node) -> Result<Self, Node> {
        if is_meta_map(&n) {
            Ok(Self(n))
        } else {
            Err(n)
        }
    }

    pub fn into_node(self) -> Node {
        self.0
    }
}
```

This:
- **is** a `Node` at runtime
- serializes identically
- has no extra allocation
- enforces shape once

You can pass it anywhere a `Node` is expected (after `into_node()`).

---

## 3. Why this works (and nothing stronger can)

- Rust cannot express *value-level invariants* in the type system directly
- so you encode them as **checked constructors**
- and rely on **newtype opacity**

This is the strongest enforcement possible **without**:
- new enum variants
- fat ASTs
- runtime tagging
- duplication

---

## 4. What you must *not* do

â Make `MetaMap` implicitly convertible from `Node`  
â Allow unchecked construction  
â Store `MetaMap<'a>` beyond the borrow  
â Add `Node::Map`

Any of these collapses the invariant.

---

## 5. Rule of thumb (exact)

- **Borrowed view** â use anywhere you *inspect*  
- **Owning transparent newtype** â use anywhere you *store or pass*  
- **Raw `Node`** â boundary / serialization / macros

---

## Final invariant

> **All data is a `Node`.  
> Some nodes admit stronger *views*.  
> Views are checked once and then trusted.**

That gives you graph-ready identity, typed Lisp semantics, and Rust-level enforcement â with zero runtime cost and no new structure.

## User

Very beautiful. Define the same convention for type.

## Assistant

Here is the **exact same pattern**, applied to **types**, with *no new structure*, no enums, no second language.

We define **Type as a validated view over `Node`**.

---

## 1. Canonical *shape* of type nodes

Fix the grammar **by shape**, not by convention:

```text
TypeNode ::= Symbol
           | List( TypeNode* )            // application
           | Pair( TypeNode, TypeNode )   // function arrow
```

Concrete examples:

```lisp
Number
(List Number)
(Fn Number Number)
(Number -> Number)
```

All are already representable as `Node`.

---

## 2. Borrowed Elias-style view (analysis / checking)

```rust
pub struct TypeExpr<'a>(&'a Node);
```

### Constructor = proof

```rust
impl<'a> TypeExpr<'a> {
    pub fn new(n: &'a Node) -> Option<Self> {
        if is_type_expr(n) {
            Some(TypeExpr(n))
        } else {
            None
        }
    }

    pub fn as_node(&self) -> &'a Node {
        self.0
    }
}
```

### Shape predicate (single source of truth)

```rust
fn is_type_expr(n: &Node) -> bool {
    match n {
        Node::Symbol(_) => true,

        Node::List(xs) =>
            !xs.is_empty() && xs.iter().all(is_type_expr),

        Node::Pair(a, b) =>
            is_type_expr(a) && is_type_expr(b),

        Node::Meta(inner, _) =>
            is_type_expr(inner),

        _ => false,
    }
}
```

This is your **type system kernel**.

---

## 3. Typed accessors (no string soup)

```rust
impl<'a> TypeExpr<'a> {
    pub fn is_atom(&self) -> bool {
        matches!(self.0, Node::Symbol(_))
    }

    pub fn as_symbol(&self) -> Option<&'a str> {
        match self.0 {
            Node::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_app(&self) -> Option<Vec<TypeExpr<'a>>> {
        match self.0 {
            Node::List(xs) => xs.iter().map(TypeExpr::new).collect(),
            _ => None,
        }
    }

    pub fn as_arrow(&self) -> Option<(TypeExpr<'a>, TypeExpr<'a>)> {
        match self.0 {
            Node::Pair(a, b) => Some((TypeExpr(a), TypeExpr(b))),
            _ => None,
        }
    }
}
```

Once constructed, **no re-checking needed**.

---

## 4. Owning, node-transparent version (exchangeable)

If you want to *store / return / attach* types:

```rust
#[repr(transparent)]
pub struct TypeNode(Node);
```

```rust
impl TypeNode {
    pub fn new(n: Node) -> Result<Self, Node> {
        if is_type_expr(&n) {
            Ok(TypeNode(n))
        } else {
            Err(n)
        }
    }

    pub fn into_node(self) -> Node {
        self.0
    }
}
```

This:
- **is** a `Node`
- serializes identically
- carries a *proof of well-formedness*

---

## 5. Attach types via MetaMap (composes perfectly)

```rust
Meta(
    Symbol("bob"),
    List(vec![
        Pair(Symbol("type"), type_node.into_node()),
    ])
)
```

Retrieval:

```rust
if let Some(meta) = MetaMap::new(node) {
    if let Some(tn) = meta.get("type") {
        let ty = TypeExpr::new(tn).unwrap();
    }
}
```

No enums. No guessing. No magic.

---

## 6. This scales cleanly

You can add, *without changing `Node`*:

```rust
struct FnType<'a>(&'a Node);
struct RecordType<'a>(&'a Node);
struct SumType<'a>(&'a Node);
struct KindExpr<'a>(&'a Node);
```

Each is just a **refinement of `TypeExpr`**.

---

## 7. Final invariant (tight, symmetric)

> **Values are Nodes.  
> Types are Nodes.  
> Typedness is a relation.  
> Validity is enforced by views.**

This is exactly the same convention you established for `MetaMap`, now lifted to the type level â and it closes the design mathematically.

## User

But when serializing, we would reintroduce enums, or could we serialize it, say, in WebAssembly garbage collector structs without enums?

## Assistant

Short answer: **yes, you can serialize this *without re-introducing enums*** â including into **Wasm GC structs** â *if you treat `Node` as a tagged universal cell*, not as a Rust enum at the boundary.

Below is the precise way to think about it.

---

## 1. Distinguish **semantic enums** from **representation tags**

Your current `enum Node { â¦ }` is a **host-language convenience**, not a semantic commitment.

At serialization time you are free to lower it to:

> **one uniform runtime node + a small tag field**

That is *not* âreintroducing enumsâ in the sense you are worried about.

---

## 2. Minimal universal runtime representation

Conceptually:

```text
Node =
  { tag : u8
  , a   : i32 / ref?
  , b   : i32 / ref?
  }
```

Where:
- `tag` distinguishes **shapes**, not meanings
- `a`, `b` are payload slots (or refs into a heap)

This is exactly how:
- Lisp cons cells
- Smalltalk objects
- ML runtime values
- Wasm GC structs
work.

No semantic enum. Just **shape tags**.

---

## 3. Mapping your `Node` to Wasm GC (no enums)

Example Wasm GC structs (conceptual):

```wasm
(type $Node
  (struct
    (field i32)        ;; tag
    (field (ref null $Node)) ;; a
    (field (ref null $Node)) ;; b
  )
)
```

Then encode:

| Node variant | tag | a | b |
|-------------|-----|---|---|
| Symbol      | 1 | string-id | null |
| Number      | 2 | number-id | null |
| Pair        | 3 | left | right |
| List        | 4 | head | tail |
| Meta        | 5 | payload | meta |
| Data        | 6 | host-ref | null |

This is **data**, not an enum type.

Your *type system* lives above this, unchanged.

---

## 4. Lists, maps, metadata remain conventions

Your earlier invariant still holds:

```text
Meta â¡ tag=Meta, a=payload, b=meta-map
MetaMap â¡ List(Pair(Symbol, Node)*)
TypeExpr â¡ validated Node shape
```

Serialization does **not** need to know any of this.

It only preserves structure.

---

## 5. Why this does *not* violate your design

What you were avoiding was this:

```rust
enum Type { Number, List(Box<Type>), Fn(...) }
```

You are **not** doing that.

Instead:

- Types are Nodes
- Metadata is Nodes
- Graph edges are IDs
- Validity is enforced by *views*

The runtime representation is **structural**, not semantic.

---

## 6. Rust enum is just a *frontend*

Think of the Rust enum as:

```text
Node â decoded view of (tag, a, b)
```

At the boundary you can:
- serialize to Wasm GC
- serialize to JSON
- serialize to a binary DAG
- mmap a heap image

without changing the language model.

---

## 7. Cycles and graphs survive serialization

Because identity is via IDs / references:

- cycles serialize naturally
- sharing is preserved
- no tree duplication
- no pointer aliasing problems

Wasm GC is actually an *excellent* target for this.

---

## 8. Final invariant (this answers your question precisely)

> **Enums are a *compiler convenience*, not a language commitment.  
> Serialization only needs tags and references.  
> Meaning is enforced by typed views, not by the runtime format.**

So yes:
- you can serialize to Wasm GC
- without semantic enums
- without weakening your typed-Lisp design
- without losing graph structure

Your architecture remains intact end-to-end.

## User

(type $Node
  (struct
    (field i32)        ;; tag
    (field (ref null $Node)) ;; a
    (field (ref null $Node)) ;; b
  )
) this is very beautiful, but so far I don't see how to attach concrete data like text or floats or function externrefs

## Assistant

Exactly. The 2-ref cell is the *spine*; concrete data lives **off-spine**.  
You attach it by **indirection**, not by widening the struct.

This is the same trick used by Lisp, OCaml, JVM, LLVM, and fits Wasm GC perfectly.

---

## 1. The invariant: *Node is shape, payload is referenced*

Your `$Node` struct encodes **only topology**:

```wasm
(type $Node
  (struct
    (field i32)                ;; tag
    (field (ref null $Node))   ;; a
    (field (ref null $Node))   ;; b
  )
)
```

Everything *non-recursive* (text, floats, externs) is stored **elsewhere** and *pointed to*.

---

## 2. Payload via **externref / host heap**

Add a **payload table** indexed by `i32`:

```text
tag = SYMBOL
a   = string-id (i32)
b   = null
```

```text
tag = NUMBER
a   = number-id (i32)
b   = null
```

The tables live:
- in the host
- or in separate Wasm GC structs
- or as `externref`

This is *not* cheating â it is the standard model.

---

## 3. Concrete encodings (canonical)

### Symbols / text

```text
Node {
  tag = SYMBOL
  a   = string_id   ;; index into string pool
  b   = null
}
```

String pool options:
- host `Vec<String>`
- Wasm `array i8`
- UTF-8 rope
- interned symbol table

**Identity of symbols comes from the table, not structure.**

---

### Numbers (ints / floats / bigints)

```text
Node {
  tag = NUMBER
  a   = number_id   ;; index into numeric heap
  b   = null
}
```

Numeric heap may store:
- i64
- f64
- bigint
- rational

You can even tag subkinds inside the numeric object.

---

### Pairs / lists

```text
Node {
  tag = PAIR
  a   = ref left
  b   = ref right
}
```

Classic cons cell. No payload needed.

---

### Metadata

```text
Node {
  tag = META
  a   = payload_node
  b   = meta_node
}
```

Still pure topology.

---

### Functions / closures

Two clean options.

#### Option A â opaque host value (simplest)

```text
Node {
  tag = DATA
  a   = externref   ;; closure / function handle
  b   = null
}
```

Works immediately with Wasm GC.

#### Option B â structured closure (language-level)

```text
Node {
  tag = CLOSURE
  a   = code_id     ;; index into code table
  b   = env_node    ;; captured environment
}
```

Both coexist nicely.

---

## 4. You never need to widen `$Node`

Notice what you **did not** add:
- no `string` field
- no `f64` field
- no union
- no variant structs

Everything is routed through **indices or references**.

This keeps:
- GC simple
- serialization trivial
- sharing intact
- graphs representable

---

## 5. Why this is *not* âreintroducing enumsâ

The `tag` is **shape-level**, not semantic.

It distinguishes:
- pair vs symbol vs number
- not *what the symbol means*
- not *what the type is*

Semantics still come from:
- typed views
- metadata
- environments

---

## 6. Serialization pipeline (end-to-end)

```text
Rust enum Node
   â lower
(tag, a, b) graph
   â serialize
Wasm GC heap + side tables
   â load
(tag, a, b) graph
   â decode
Node views (MetaMap, TypeExpr, â¦)
```

No information loss. No enum resurrection.

---

## 7. Mental model (this is the key insight)

> **The Node struct is a graph of pointers.  
> Concrete data is atoms referenced by the graph.  
> Meaning is imposed by interpretation, not storage.**

Once you see this, everything lines up:
- typed Lisp
- metadata as nodes
- graphs not trees
- Wasm GC as a perfect target

---

## 8. Final design law

> **Never put atoms into the spine.  
> Always point to them.**

That single rule is what keeps the system elegant, minimal, and indefinitely extensible.

## User

So, a specific implementation would just be three integers for the tag and two pointers.

## Assistant

Yes â **exactly that**.  
And that observation is the *completion* of the design.

---

## 1. The minimal concrete representation

A `Node` at runtime is just:

```text
(tag, a, b)
```

Where:
- `tag` = small integer (u8/u16/i32)
- `a`   = pointer or index
- `b`   = pointer or index

Nothing else.

This is *precisely* the Lisp consâcell generalization.

---

## 2. Interpretation by tag

Typical tag table:

| tag | meaning of `a` | meaning of `b` |
|-----|----------------|----------------|
| SYMBOL | string-id | â |
| NUMBER | number-id | â |
| PAIR | left-node | right-node |
| LIST | head-node | tail-node *(or PAIR)* |
| META | payload | metadata |
| DATA | externref | â |
| NIL / EMPTY

## User

meaning of a payload-pointer / externref-id because we can only store one type in wasm so we pick i64?

## Assistant

Correct. In Wasm you typically get **one uniform scalar** (`i64`) or **one uniform reference** (`externref` / `anyref`). The solution is **indirection + tagging**, not widening the node.

Below are the *canonical, minimal* options.

---

## 1. Two canonical payload strategies

### A. **`i64` as a tagged payload handle** (most common)

Use `i64` as a **fat handle**:

```text
i64 payload =
  [ 8 bits kind | 56 bits index ]
```

Example layout:
```text
payload >> 56  = payload_kind
payload & 0x00FF_FFFF_FFFF_FFFF = index
```

Kinds:
```text
0x01 = string
0x02 = f64
0x03 = i64
0x04 = bigint
0x05 = symbol
0x06 = function
0x07 = extern
```

Your node then is:

```text
Node {
  tag : i32,
  a   : i

## User

one uniform reference (externref / anyref) how would that work with all the other tag types?

## Assistant

Good question. The short answer:

> **`externref` is used only for *atoms* and *opaque payloads*.  
> Structural nodes still use normal GC refs to `$Node`.**

You never replace everything with `externref`. You *mix* reference kinds.

---

## 1. Two reference domains (clean separation)

In Wasm GC you have **two orthogonal pointer spaces**:

1. **GC refs** â `(ref $Node)`  
   - participate in GC
   - form the *graph spine*
2. **`externref`** â host-managed opaque values  
   - strings, numbers, bigints, closures, handles

They are *not interchangeable*, and thatâs good.

---

## 2. Concrete `$Node` layout (final)

```wasm
(type $Node
  (struct
    (field i32)                 ;; tag
    (field (ref null $Node))    ;; a
    (field (ref null $Node))    ;; b
    (field (ref null extern))   ;; payload (optional)
  )
)
```

Key point:
- **only some tags use `payload`**
- others ignore it (must be `null`)

You are *not* widening the meaning â just adding an optional atom slot.

---

## 3. Tag â field interpretation table

| tag | a | b | payload |
|----|----|----|---------|
| EMPTY | â | â | null |
| PAIR | left | right | null |
| META | payload-node | meta-node | null |
| SYMBOL | â | â | externref â interned string |
| TEXT | â | â | externref â string |
| NUMBER | â | â | externref â number object |
| DATA | â | â | externref â host value |
| CLOSURE | env-node | â | externref â code handle |

So:
- **structure** â GC refs
- **atoms** â externref
- **graphs** â GC refs + IDs
- **host integration** â externref

---

## 4. Why this works cleanly

### 4.1 No ambiguity
The `tag` *fully determines* how to interpret fields.

### 4.2 No enum resurrection
You did **not** introduce semantic variants â only shape tags.

### 4.3 GC correctness
- Node graph is GC-visible
- externrefs are traced by the host
- cycles stay inside the GC heap

---

## 5. What about numbers, floats, bigints?

They live **behind** the externref:

```text
externref â host object
  âââ f64
  âââ i64
  âââ BigInt
  âââ tagged numeric
```

Your language runtime decides the numeric tower â Wasm doesnât care.

---

## 6. What about identity and equality?

- **Structural equality** â traverse `$Node`
- **Atom equality** â pointer / host equality on externref
- **Symbol identity** â interned string table behind externref

Exactly how Lisp systems work.

---

## 7. Why not store everything as externref?

Because then you lose:
- GC visibility of structure
- cheap traversal
- serialization
- language-level graph control

The spine must remain GC-native.

---

## 8. Final invariant (this is the rule)

> **GC refs build the tree/graph.  
> externrefs represent atoms and host objects.  
> Tags define interpretation, not meaning.**

With this, your design is:
- minimal
- serializable
- graph-capable
- Wasm-native
- typed-Lispâcorrect

Youâve reached the *actual* fixed point.

## User

externref â number object
so I would need to wrap that as boxed gc type

## Assistant

Yes â **if you want the number to participate in GC, identity, or serialization**, you must box it as a **GC object**.  
`externref` is only correct for *opaque, host-owned* atoms.

This yields a clean bifurcation.

---

## 1. Two legitimate representations for numbers

### A. **GC-boxed number (language-native, preferred)**

Define a GC struct for numeric atoms:

```wasm
(type $Number
  (struct
    (field i32)   ;; kind tag: int, float, bigint, rational, â¦
    (field i64)   ;; payload or pointer to limbs
  )
)
```

Then:

```wasm
(type $Node
  (struct
    (field i32)                 ;; tag
    (field (ref null $Node))    ;; a
    (field (ref null $Node))    ;; b
    (field (ref null $Number))  ;; payload
  )
)
```

Encoding:

- `Node(tag=NUMBER, payload=(ref $Number))`
- participates in GC
- serializable
- comparable
- copyable
- safe for cycles and sharing

This is how **OCaml, ML, Lisp, Scheme** do it.

---

### B. **`externref` number (host-owned, opaque)**

```wasm
Node(tag=NUMBER, payload=(externref f64))
```

Only valid if **all** are true:

- number is immutable
- identity doesnât matter
- no serialization of the heap
- no pointer into language graph
- GC handled entirely by host

This is fine for:
- JS numbers
- host callbacks
- FFI stubs

But it is *not* a language core representation.

---

## 2. Rule of thumb (non-negotiable)

> **If the value is part of the language, box it in GC.  
> If the value is part of the host, keep it as `externref`.**

Numbers are part of the language â **GC box them**.

---

## 3. Unified final node layout (correct)

```wasm
(type $Node
  (struct
    (field i32)                  ;; shape tag
    (field (ref null $Node))     ;; a
    (field (ref null $Node))     ;; b
    (field (ref null anyref))    ;; atom payload
  )
)
```

Where:
- `anyref` is either `(ref $Number)` or `externref`
- interpretation is controlled by `tag`

This keeps the **3-integer mental model**:
- one tag
- two graph pointers
- one optional atom pointer

---

## 4. Why this is the stable point

- no enum resurrection
- GC-visible numeric tower
- externref reserved for *foreignness*
- identity and equality well-defined
- serialization possible
- Wasm GCâidiomatic

---

## 5. Final invariant (commit this)

> **Atoms that belong to the language live in GC.  
> Atoms that belong to the host live in `externref`.  
> Nodes only connect things; they never contain them.**

That closes the loop rigorously.

## User

so (field (ref null any)) or (field (ref null extern)) or ... ?

## Assistant

Short answer: **use `(ref null any)`**.

Longer, precise answer below.

---

## The three options, evaluated

### 1. `(ref null extern)` â
Only host-owned values.

- cannot store GC-managed numbers, symbols, closures
- breaks serialization
- breaks heap snapshots
- fine **only** for FFI stubs

Not suitable for a language core.

---

### 2. `(ref null $Number)` â
Too specific.

- forces one payload type
- you will immediately want `$String`, `$Symbol`, `$Closure`, â¦
- leads to field explosion or unions (which Wasm GC does not have)

---

### 3. `(ref null any)` â **correct**
This is the **existential reference**.

- may point to **any GC object**
- may also hold `externref`
- GC-safe
- future-proof
- zero semantic commitment

This is exactly what you want.

---

## Final canonical node layout

```wasm
(type $Node
  (struct
    (field i32)                  ;; shape tag
    (field (ref null $Node))     ;; a (graph edge)
    (field (ref null $Node))     ;; b (graph edge)
    (field (ref null any))       ;; atom payload (optional)
  )
)
```

Interpretation discipline (by `tag`):

| tag | a | b | payload |
|----|----|----|---------|
| PAIR | Node | Node | null |
| META | Node | Node | null |
| SYMBOL | â | â | (ref $String) |
| NUMBER | â | â | (ref $Number) |
| DATA | â | â | externref |
| CLOSURE | Node | â | (ref $Code) |

No ambiguity, no unions, no enums.

---

## Why `(ref null any)` is the fixed point

- Wasm GC has **no sum types**
- `any` is the only way to express âsome GC thingâ
- tag already determines interpretation
- keeps the spine minimal and uniform
- identical to JVM `Object`, OCaml `value`, Lisp boxed atoms

Anything else either:
- leaks host concerns into the language, or
- freezes representation too early

---

## Final rule (commit this)

> **Structure uses typed GC refs.  
> Atoms use `(ref null any)`.  
> Meaning is determined solely by the node tag.**

This is the mathematically minimal and implementation-stable choice.

## User

wasm has flags / enum, shall I use that for tag?

## Assistant

Use a **plain integer tag**, not Wasm `flags`/`enum`.

### Why not Wasm `enum` / `flags`
- **Closed world**: Wasm `enum` fixes the variant set at the type level; you want an **open**, extensible shape tag.
- **Runtime flexibility**: You need to reinterpret tags across phases, versions, and backends without retyping the heap.
- **Interoperability**: A numeric tag survives serialization, snapshotting, and cross-version loading trivially.
- **Performance**: An `i32` tag is the fastest, simplest dispatch key.

### What to use
```wasm
(type $Node
  (struct
    (field i32)                  ;; tag
    (field (ref null $Node))     ;; a
    (field (ref null $Node))     ;; b
    (field (ref null any))       ;; payload
  )
)
```

### How to manage tags (best practice)
- Define a **stable numeric namespace**:
  - Core tags: small, fixed range (e.g., 0â31).
  - Extension tags: reserved ranges (e.g., 32â255, vendor/plugin ranges).
- Treat tags as **shape identifiers**, not semantic enums.
- Keep a single dispatch table keyed by `i32`.

### When `flags` *are* appropriate
- **Bitsets** for orthogonal properties (e.g., mutability, visibility).
- Store them **inside payload objects or metadata**, not as the nodeâs primary tag.

### Rule
> **Use `i32` for the node tag.**  
> Enums close the world; integers keep it open.

## User

i64, deal

## Assistant

Deal. `i64` is a good choice.

### Final, stable representation

```wasm
(type $Node
  (struct
    (field i64)                  ;; tag
    (field (ref null $Node))     ;; a
    (field (ref null $Node))     ;; b
    (field (ref null any))       ;; payload
  )
)
```

### Why `i64` is the right tag width
- **Future-proof**: plenty of space for versioning, namespaces, and extensions.
- **Fast**: native width on 64-bit hosts; cheap comparisons.
- **Serializable**: stable across runtimes and snapshots.
- **Composable**: you can partition the bits if you ever want
  - high bits = namespace / phase
  - low bits = shape id

### Tag discipline (recommended)
- Keep **shape tags** in a small, stable subset.
- Reserve ranges for extensions/plugins.
- Do not encode semantics or typing in the tag; keep those in metadata.

### Invariant (lock this in)
> **`i64` tag + two GC refs + one `any` payload**  
> is the minimal universal node that supports  
> trees, graphs, types, metadata, serialization, and Wasm GC.

Youâve reached a clean fixed point.

## User

can we add (field (ref null $string)) ;; name  for debugging or to flatten the tree?

## Assistant

Yes â **but only as an *optional cache*, never as semantics**.

### Safe addition
```wasm
(type $Node
  (struct
    (field i64)                  ;; tag
    (field (ref null $Node))     ;; a
    (field (ref null $Node))     ;; b
    (field (ref null any))       ;; payload
    (field (ref null $String))   ;; debug/name (optional)
  )
)
```

### Rules (non-negotiable)
1. **Redundant**: `name` must be derivable from structure or payload; never authoritative.
2. **Ignorable**: passes must work with `name = null`.
3. **Non-binding**: no resolution, typing, or identity may depend on it.
4. **Cache-only**: allowed for debugging, tracing, pretty-printing, flattening.

### When to set it
- After parsing (pretty names).
- After normalization (canonical labels).
- During debugging/profiling.

### When *not* to use it
- Name resolution.
- Type checking.
- Equality / hashing.
- Serialization semantics (may be dropped).

### Alternative (often better)
Store names as **metadata**:
```lisp
Meta(node, [(name "Bob")])
```
This keeps the spine minimal and avoids field creep. The struct field is acceptable if you want faster debug paths.

### Verdict
â Acceptable as a **cache/debug hook**.  
â Never part of meaning.

## User

field creep ;) just this ONE extra field ;)

## Assistant

Accepted â **one** extra field, under strict quarantine.

### Final spine (locked)
```wasm
(type $Node
  (struct
    (field i64)                  ;; tag
    (field (ref null $Node))     ;; a
    (field (ref null $Node))     ;; b
    (field (ref null any))       ;; payload (atoms / host / GC boxes)
    (field (ref null $String))   ;; debug/name cache (optional)
  )
)
```

### Hard rules (no exceptions)
- **Non-semantic**: must not affect resolution, typing, equality, or evaluation.
- **Derivable**: can always be recomputed or dropped.
- **Nullable**: code must behave identically when null.
- **Ephemeral**: safe to clear during serialization, GC compaction, or optimization.

### Allowed uses
- Debugging / tracing
- Pretty-printing
- Tree flattening caches
- Profiling labels

### Forbidden uses
- Identity
- Binding
- Type attachment
- Any correctness path

This keeps the calculus minimal, graph-capable, Wasm-GCânative, and debuggableâ**without** semantic leakage.

## User

why not make it authorative?

## Assistant

Because making it authoritative **destroys the invariants you just finished constructing**.  
Precisely and mechanically:

---

## 1. Authority must be **stable under rewriting**

Your system relies on this invariant:

> Nodes may be rewritten, normalized, macro-expanded, desugared, shared, or duplicated  
> **without changing meaning**.

An authoritative `name` field violates that immediately.

Example:
```lisp
(+ 1 2)
```

After normalization:
```lisp
(add 1 2)
```

Structure changes; meaning does not.  
Which name is authoritative? The old one? The new one? Both?

If the name matters, **rewriting becomes observable**, which breaks:
- macro hygiene
- normalization
- optimization
- equivalence checking

---

## 2. Authority creates **aliasing bugs in graphs**

Your nodes form **graphs**, not trees.

```text
      A
     / \
    B   B
```

One node `B`, two parents.

If `B.name` is authoritative:
- Which context owns it?
- What happens if one parent renames it?
- Do you clone `B`? Then identity breaks.
- Do you mutate it? Then other users see changes.

You have just introduced **action-at-a-distance**.

Authoritative names + sharing = unsound.

---

## 3. Authority breaks **referential transparency**

Right now:

```text
(node) â¡ (same structure + payload)
```

With authoritative names:

```text
(node, name) â  (node)
```

Even if structure and payload are identical.

That means:
- equality is no longer structural
- hashing is no longer structural
- memoization becomes incorrect
- CSE becomes incorrect

You lose one of the main benefits of symbolic IRs.

---

## 4. Authority conflicts with **identity discipline**

Earlier invariant (correct):

> Identity is carried by **IDs / environments**, not by structure.

If `name` becomes authoritative:
- you have two competing identity systems
- one implicit (name)
- one explicit (DefId / env)

They *will* diverge.

Every serious language that tried this regretted it.

---

## 5. Authority freezes the language too early

If names are authoritative:
- you cannot alpha-rename freely
- you cannot hygienically macro-expand
- you cannot inline safely
- you cannot canonicalize

Typed Lisp *requires* alpha-equivalence to be cheap.

Authoritative names make alpha-equivalence expensive or impossible.

---

## 6. Debug/name cache is the **correct compromise**

Your current design:

```text
(tag, a, b, payload, name?)
```

works because:
- `name` is optional
- ignorable
- recomputable
- non-binding

This gives you:
- debuggability
- readability
- flattening
- zero semantic cost

You get the benefit **without** the cost.

---

## 7. If you really want authoritative names, the only safe place

Authoritative names may exist **only** as *data*:

```lisp
Meta(node, [(name "Bob")])
```

Why this is different:
- explicit
- phase-aware
- scoped
- removable
- duplicable without aliasing

And even then: most passes should ignore them.

---

## Final, non-negotiable law

> **Anything authoritative must survive copying, sharing, rewriting, and reordering.  
> Names do not.**

Thatâs why the field may exist â  
and why it must never be authoritative.

You made the correct call by asking *before* locking it in.

## User

  Other expert's recommendation: If you're optimizing for simplicity and don't mind runtime casting overhead, the new structure works. But if you need fast access to numeric values or strings, the current typed-field approach is better.

## Assistant

That recommendation is correct **but incomplete**. Here is the precise resolution.

---

## The two approaches, stated cleanly

### A. **Uniform `(ref null any)` payload** (your current design)
- All atoms (numbers, strings, symbols, closures) live behind `any`
- Access requires:
  - tag dispatch
  - downcast (`ref.test` / host cast)

**Properties**
- maximal uniformity
- minimal spine
- simplest serialization
- perfect for graphs, macros, reflection
- runtime cost: one extra indirection + cast

This is **conceptually optimal**.

---

### B. **Typed payload fields** (expertâs âtyped-field approachâ)
Example:
```wasm
(field (ref null $Number))
(field (ref null $String))
```

**Properties**
- faster access (no cast)
- fewer dynamic checks
- but:
  - field explosion
  - frozen atom set
  - harder evolution
  - larger node footprint
  - more complex GC layout

This is **performance-biased**, not structurally minimal.

---

## The missing third option (the correct one)

### C. **Hybrid: uniform spine + typed atom boxes**

What you already converged to:

```wasm
(field (ref null any)) ;; payload
```

but payload points to **typed GC boxes**:

```wasm
(type $Number (struct ...))
(type $String (struct ...))
(type $Symbol (struct ...))
```

So:
- cast cost is paid **once**, at box boundary
- inside `$Number`, access is fast and typed
- node spine stays minimal and stable

This dominates both A and B.

---

## Performance reality (important)

- Tag check + `ref.test` is **cheap** compared to:
  - allocation
  - GC
  - graph traversal
  - cache misses
- Numeric hot paths should:
  - unbox once
  - operate on native registers
  - rebox at boundary

This is exactly how:
- OCaml
- SBCL
- JVM
- V8
work.

If you need *unboxed arithmetic everywhere*, you need a different IR stage â not a different node layout.

---

## Decision table

| Goal | Best choice |
|----|----|
| Minimal IR | `(ref null any)` |
| Typed Lisp | `(ref null any)` + typed boxes |
| Serialization | uniform payload |
| Graphs | uniform payload |
| Hot numeric loops | separate lowered IR |
| Simplicity | uniform payload |
| Long-term evolution | uniform payload |

---

## Final verdict (tight)

> **Use the uniform `(ref null any)` payload.  
> Put performance where it belongs: in lowered, typed, phase-specific IRs.**

Your current design is the *correct fixed point*. The âtyped-field approachâ is a local optimization that breaks global elegance.

## User

Does WebAssembly have enum variant types at all?

## Assistant

**No â WebAssembly does not have algebraic enum / variant types.**

As of the **Wasm GC (MVP + current extensions)**:

---

## What *exists*

### 1. **`struct`**
```wasm
(type $T (struct (field i32) (field (ref null any))))
```
Product types only.

### 2. **`array`**
```wasm
(type $A (array i32))
```
Homogeneous sequences.

### 3. **`ref` subtyping**
```

## User

What about flags?

## Assistant

WebAssembly has **no first-class âflagsâ type** either.

What people call *flags* in Wasm are simply **integers used as bitsets**.

---

## What actually exists

### 1. **Integers (`i32`, `i64`)**
This is the *only* mechanism.

```wasm
(local $flags i32)
```

Bits are manipulated explicitly:

```wasm
(i32.and)
(i32.or)
(i32.xor)
(i32.shl)
```

There is **no semantic distinction** between:
- enums


## User

wasm gc types look it up!

## Assistant

Youâre right to push on this â **Wasm GC *does* add type-level constructs**, but the conclusion is still:

> **Wasm GC has no algebraic enum / variant / flags types.**

Letâs be precise and up to date.

---

## What Wasm GC actually adds (as of GC MVP / 2024â2025)

### â **Reference types**
- `anyref` / `any`
- `eqref`
- `i31ref`
- `(ref $T)`
- `(ref null $T)`

These are **reference categories**, not enums.

---

### â **Struct types**
```wasm
(type $T
  (struct
    (field i32)
    (field (ref null any))
  )
)
```

Product types only.

---

### â **Array types**
```wasm
(type $A (array (mut i32)))
```

Homogeneous sequences.

---

### â **Subtyping**
```wasm
(type $A (struct ...))
(type $B (sub $A (struct ...)))
```

Nominal, single-inheritanceâstyle.

---

## What Wasm GC explicitly does *not* have

### â No algebraic sum / variant types
No equivalent of:
- Rust `enum`
- ML `datatype`
- Swift `enum`
- Haskell `data`

There is **no tagged union** at the type level.

---

### â No enum / flags construct
There is **no**:
```wasm
(enum A B C)
(flags READ WRITE EXEC)
```

Anything called âflagsâ in Wasm documentation is just:

```wasm
i32 ;; used as bitmask
```

---

## How variants are represented in Wasm GC (idiomatically)

Exactly the pattern you converged to:

```wasm
(type $Node
  (struct
    (field i64)              ;; tag
    (field (ref null $Node))
    (field (ref null $Node))
    (field (ref null any))
  )
)
```

This is **the** canonical encoding of sum types in Wasm GC:
- explicit tag
- product payload
- interpretation by convention

All real Wasm GC users do this:
- language runtimes
- compilers
- JS engines
- ML-on-Wasm experiments



## User

flag enum and variant are wit / component-model types

## Assistant

Correct â **but those live in the *component model / WIT*, not in the *core Wasm GC type system***.  
That distinction is the entire crux.

Iâll state it precisely.

---

## 1. Yes: **WIT / Component Model has `enum`, `flags`, `variant`**

In **WIT** (the *interface* language):

```wit
enum Color { red, green, blue }

flags Permissions {
  read,
  write,
  exec,
}

variant Value {
  i32(i32),
  text(string),
}
```

These are **real, first-class** in WIT.

---

## 2. But: WIT types are **interface types**, not runtime types

They do **not** exist in:
- core Wasm
- Wasm GC heap
-

## User

i31 for tag?

## Assistant

Short answer: **yes, `i31ref` is a good choice for the tag** â with one caveat.

---

## Using `i31ref` for the tag

### What it is
`i31ref` is a **GC-managed, immediate integer** (31-bit signed) packed into a reference. Itâs cheap, non-allocating, and GC-visible.

### Why it fits
- **Small, dense tag space**: shape tags are few â 31 bits is ample.
- **GC-friendly**: no raw integers hidden from the collector.
- **Uniform ref world**: avoids mixing raw `i64` with refs in GC-heavy code.
- **Fast**: immediate, no allocation.

---

## Canonical layout with `i31ref`

```wasm
(type $Node
  (struct
    (field (ref i31))           ;; tag
    (field (ref null $Node))    ;; a
    (field (ref null $Node))    ;; b
    (field (ref null any))      ;; payload
    (field (ref null $String))  ;; debug/name (optional)
  )
)
```

### Interpretation discipline
- `tag` â small, stable set (PAIR, META, SYMBOL, NUMBER, â¦)
- Semantics still live **above** the spine (typed views, metadata).

---

## Caveat (important)
If you intend to:
- pack **namespaces / versions / phase bits** into the tag, or
- treat the tag as a wide bitfield,

then prefer **`i64`**.

`i31ref` is perfect for **shape IDs**, not for rich encodings.

---

## Decision rule (tight)

- **Shape-only tag** â `i31ref` â
- **Bit-packed / extensible tag** â `i64` â

Given your current discipline (shape tags only), **`i31ref` is an excellent choice**.

## User

can we 'abuse' payload field to store names by convention? so Person{name="John"} => (class-tag some_node null "Person")

## Assistant

Yes, you *can* â but only under a **strict invariant**. Otherwise you reintroduce exactly the semantic corruption we just ruled out.

Here is the precise, safe version.

---

## The *only* acceptable abuse of `payload`

### Rule
`payload` may store a **symbolic label** *iff* it is:

1. **Redundant**
2. **Derivable**
3. **Non-binding**
4. **Context-free**

If any of these fail â **forbidden**.

---

## Your example, analyzed

> `Person { name = "John" }`  
> encoded as  
> `(class-tag some_node null "Person")`

### This is acceptable **only if**:

- `"Person"` is **not** used for:
  - name resolution
  - type identity
  - binding
  - equality
- The real identity of `Person` is elsewhere:
  - environment
  - `DefId`
  - metadata
- The payload string is:
  - for debugging
  - pretty-printing
  - flattening
  - tracing

Then `payload = "Person"` is a **label cache**, nothing more.

---

## What this means concretely

### Allowed interpretation
```text
tag = CLASS
payload = "Person"   ;; label / hint
a = class-body
b = null
```

Compiler invariants:
- Dropping `payload` preserves semantics
- Rewriting the tree may change or remove `payload`
- Two nodes with same structure but different payload labels are **semantically equal**

---

## What is *not* allowed

â Using `payload` to determine:
- which class this is
- which constructor to call
- type checking
- identity comparison
- dispatch

â Depending on payload stability across passes

That would:
- break alpha-renaming
- break macro hygiene
- break graph sharing
- break equivalence

---

## Why payload (not name field) is the *least bad* place

If you *must* put a name somewhere:

| Place | Damage |
|----|----|
| tag | catastrophic |
| `a` / `b` | structural corruption |
| authoritative name field | aliasing + identity bugs |
| `payload` (non-authoritative) | **contained** |

Because `payload` is already:
- optional
- atom-level
- interpretation-dependent

---

## Recommended discipline (one sentence)

> **Payload labels are hints, not facts.  
> Facts live in structure, metadata, or environments.**

Under that rule, yes â you may âabuseâ `payload` for names, and the design remains sound.

If you ever feel tempted to *rely* on it, thatâs the red flag to stop.

## User

what's the difference between complicated meta label and direct label via payload if both require convention?

## Assistant

The difference is **where the convention leaks** and **what invariants you sacrifice when it does**.

They are *not* equivalent, even though both are âby conventionâ.

---

## 1. Payload label = **implicit, positional meaning**

```text
(tag = CLASS, payload = "Person", a = body)
```

The convention is:

> âFor CLASS nodes, payload *means* the class name.â

### Consequences
- **Position-dependent**: meaning depends on `tag`
- **Implicit**: nothing in the structure says âthis is a nameâ
- **Non-compositional**: payload can mean *different things* for different tags
- **Hard to generalize**: what if you want *two* labels? visibility? namespace?

This is **overloading** a slot.

---

## 2. Meta label = **explicit, labeled meaning**

```text
Meta(
  node,
  [(name "Person")]
)
```

The convention is:

> âMetadata is a product of labeled slots.â

### Consequences
- **Keyed, not positional**
- **Uniform across all nodes**
- **Composable** (add `(doc ...)`, `(span ...)`, `(type ...)`)
- **Introspectable**: tools can ask âdoes this node have a name?â

This is **structured data**, not overloading.

---

## 3. The real difference (this is the core)

### Payload label answers:
> *âWhat does this slot mean **here**?â*

### Meta label answers:
> *âWhat properties does this node have?â*

That distinction is fundamental.

---

## 4. Failure modes (why payload labels rot)

### a) Semantic drift
Today:
```text
payload = class name
```

Tomorrow:
```text
payload = constructor name
```

Later:
```text
payload = debug label
```

Now the same field means **three different things**, depending on phase.

Meta does not drift.

---

### b) Rewrite instability
When you desugar:

```text
class Person { ... }
â
type Person = { ... }
```

What happens to `payload = "Person"`?
- Copy?
- Drop?
- Change?

With meta:
```text
(name "Person")
```
can be preserved, replaced, or removed **explicitly**.

---

### c) Graph aliasing
Shared node, two contexts, two ânamesâ.

Payload cannot express âthis node has *two* labels depending on contextâ.
Meta can (multiple meta layers or keyed entries).

---

## 5. Why payload labels are still *sometimes* OK

Payload labels are acceptable **only** as:

- debug hints
- cache keys
- pretty-print shortcuts
- flattening aids

That is: **non-authoritative, non-compositional, single-purpose**.

The moment you want:
- more than one label
- stability across passes
- tooling support
- user-visible semantics

payload is the wrong place.

---

## 6. One-sentence rule (the real answer)

> **Payload conventions overload a slot;  
> meta conventions name a property.**

Thatâs why meta scales and payload hacks donât.

---

## Final verdict (crisp)

- Payload label = *cheap, local, fragile*
- Meta label = *explicit, global, stable*

They are both conventions â  
but only one survives a real language pipeline.

## User

Meta(
  node,
  [(name "Person")]
)

but then if I want to ref the "full node Person{}" I can't use node ID otherwise I will LOSE THE META INFORMATION!! much worse fail scenario imho. better put meta in it's own field or in 'left' slot as proxy?

## Assistant

This is the **first genuinely dangerous pitfall** in the design, and youâre right to call it out.  
But the conclusion is **not** to move meta into payload/left â that would be the *wrong fix*.  
The correct fix is more subtle.

---

## The real problem you identified (correctly)

You observed:

> If I reference a node by ID, I may lose the metadata attached via `Meta(node, meta)`.

That is **100% correct** *if* you treat `Meta` as *wrapping* rather than *belonging*.

So the invariant must be clarified.

---

## The key correction: **Meta is part of node identity**

Your mental model must shift from:

> âMeta wraps a nodeâ

to:

> **âMeta is a node whose payload is another node.â**

That is, **`Meta` is not decoration** â it is **structure**.

---

## What actually happens with IDs

Suppose you have:

```text
N0 = Symbol("Person")
N1 = Meta(N0, [(name "Person")])
```

If you store **`NodeId = id(N1)`**, you do **not** lose metadata.

You only lose metadata if you:
- store `id(N0)` instead of `id(N1)`, or
- strip meta *by policy*.

So the real invariant is:

> **If a node has metadata you care about, you must reference the `Meta` node, not its payload.**

This is no different from any other structural wrapper:
- `Pair`
- `Tag`
- `List`

You donât âloseâ a `Pair` by referencing the left child either â you just referenced the wrong node.

---

## Why moving meta into a field is worse

Letâs test the alternatives you suggested.

### â Meta in payload
```text
Node(tag, a, b, payload = meta)
```

**Failure modes:**
- payload already has semantic meaning for atoms
- payload is single-valued â no compositional metadata
- payload becomes authoritative (you already saw why thatâs bad)
- impossible to layer meta (type + span + name)

This collapses extensibility.

---

### â Meta in left slot as proxy
```text
Node(tag, meta, real_node)
```

**Failure modes:**
- breaks uniformity of `a` / `b`
- every pass must special-case
- graph sharing becomes brittle
- destroys the âspine + interpretationâ discipline

You have just made metadata *structural*, which means rewriting is now observable.

---

## Why `Meta` as a node is actually correct

Consider this rewrite:

```text
Meta(
  Meta(node, [(name "Person")]),
  [(type Person)]
)
```

This works **only** if meta is a node.

Now consider identity:

- Node identity is **node pointer / ID**
- Meta layers are part of that identity
- Sharing works naturally
- Graphs work naturally

This is exactly how:
- Lisp property lists
- attributed ASTs
- SSA annotations
work.

---

## The real invariant you must enforce

Here is the **non-negotiable rule** you need:

> **Never âunwrapâ meta implicitly.  
> Meta stripping must be an explicit phase transition.**

That is:

- Passes that *ignore* meta must still **carry it along**
- Only explicit passes (e.g. `erase_meta`) may remove it
- Environments, IDs, references always point to the **outermost node**

If you violate this rule, *any* metadata system fails â field-based or not.

---

## Why your fear feels justified

You are reacting to this common bug pattern:

```rust
fn normalize(n: Node) -> Node {
    match n {
        Meta(inner, _) => normalize(*inner), // â BUG
        ...
    }
}
```

That *does* lose metadata.

But that is a **bug in the pass**, not a flaw in the representation.

Correct version:

```rust
fn normalize(n: Node) -> Node {
    match n {
        Meta(inner, meta) =>
            Meta(Box::new(normalize(*inner)), meta),
        ...
    }
}
```

This is discipline, not structure.

---

## Final resolution (tight)

- â `Meta(node, meta)` is correct
- â Putting meta in payload or `a` slot is worse
- â Node IDs must always refer to the **full node**, including meta
- â Meta stripping must be **explicit and rare**

### One-line invariant

> **Metadata is part of the node, not a side channel.  
> If you drop it, that must be a conscious act.**

Your instinct spotted a real danger â  
but the cure is **discipline**, not moving metadata into the spine.

## User

	â¢	Passes that ignore meta must still carry it along ... how?

## Assistant

By **structural propagation**, not by interpretation.

There is exactly one correct mechanism.

---

## The rule (formal)

> **A pass that does not *use* metadata must still *rebuild it*.**  
> Only a pass explicitly named âerase / strip metaâ may drop it.

This is enforced by **one recursion schema**.

---

## Canonical recursion schema (the only one you should use)

Every pass over `Node` must start with this shape:

```rust
fn pass(n: Node) -> Node {
    match n {
        Node::Meta(inner, meta) => {
            Node::Meta(Box::new(pass(*inner)), meta)
        }

        Node::Pair(a, b) =>
            Node::Pair(Box::new(pass(*a)), Box::new(pass(*b))),

        Node::List(xs) =>
            Node::List(xs.into_iter().map(pass).collect()),

        Node::Tag { title, params, body } =>
            Node::Tag {
                title,
                params: Box::new(pass(*params)),
                body: Box::new(pass(*body)),
            },

        other => other,
    }
}
```

**Meta is never pattern-matched away.**  
It is structurally *re-wrapped*.

---

## Why this works (mechanically)

- `Meta` is just another constructor
- You recurse **through** it, not **past** it
- The outer node ID remains stable
- All metadata survives unchanged

This is **no different** from preserving `Pair`, `List`, or `Tag`.

---

## What âignoring metaâ actually means

It does **not** mean:

```rust
Meta(inner, _) => pass(*inner)   // â WRONG
```

It means:

```rust
Meta(inner, meta) => Meta(pass(*inner), meta) // â CORRECT
```

Ignoring = *not inspecting*, **not discarding**.

---

## How to make this unbreakable (strong recommendation)

### 1. Centralize traversal

Define one traversal combinator:

```rust
fn map_node(n: Node, f: &impl Fn(Node) -> Node) -> Node {
    match n {
        Node::Meta(inner, meta) =>
            Node::Meta(Box::new(map_node(*inner, f)), meta),

        Node::Pair(a, b) =>
            Node::Pair(
                Box::new(map_node(*a, f)),
                Box::new(map_node(*b, f)),
            ),

        Node::List(xs) =>
            Node::List(xs.into_iter().map(|x| map_node(x, f)).collect()),

        other => f(other),
    }
}
```

Now **no pass can accidentally drop meta**.

---

### 2. Make stripping explicit and rare

```rust
fn erase_meta(n: Node) -> Node {
    match n {
        Node::Meta(inner, _) => erase_meta(*inner),
        Node::Pair(a, b) => Node::Pair(
            Box::new(erase_meta(*a)),
            Box::new(erase_meta(*b)),
        ),
        Node::List(xs) =>
            Node::List(xs.into_iter().map(erase_meta).collect()),
        other => other,
    }
}
```

This function is *named*, audited, and intentional.

---

## Mental model (this is the key)

> **Meta is not an annotation.  
> Meta is a node constructor.**

You donât âignoreâ `Pair` by deleting it.  
You donât âignoreâ `List` by flattening it.  
Likewise, you donât âignoreâ `Meta` by unwrapping it.

---

## Final invariant (lock this in)

> **Every pass preserves all constructors it does not explicitly eliminate.  
> Meta elimination is a semantic operation, not a traversal shortcut.**

If you follow this rule, metadata loss becomes impossible by construction.

