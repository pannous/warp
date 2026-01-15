# Normalization

Wasp accepts many syntactic forms for the same operation, but each has a **preferred canonical form**.
When users write non-canonical forms, we gently educate them about the preferred syntax.

## Philosophy

> "There should be one obvious way to do it."

We embrace syntactic flexibility for input (making the language approachable), but guide users toward consistency.
The normalization hints are shown every time a non-canonical form is used - repetition builds habit.

## Canonical Forms

### Type Casting

| Non-canonical | Canonical | Note |
|---------------|-----------|------|
| `int('123')` | `'123' as int` | Postfix `as` reads naturally |
| `str(42)` | `42 as string` | Full type name preferred |
| `String(x)` | `x as string` | Lowercase type names |
| `float(x)` | `x as float` | |
| `char(65)` | `65 as char` | |

### Variable Assignment

| Non-canonical | Canonical | Note |
|---------------|-----------|------|
| `x = 5` | `x := 5` | `:=` for definition |
| `let x = 5` | `x := 5` | No `let` keyword needed |
| `var x = 5` | `x := 5` | No `var` keyword needed |

### Function Definition

| Non-canonical | Canonical | Note |
|---------------|-----------|------|
| `function f(x) { x*2 }` | `f(x) := x*2` | Short form preferred |
| `def f(x): x*2` | `f(x) := x*2` | |
| `fn f(x) = x*2` | `f(x) := x*2` | |
| `f := it*2` | `f(x) := x*2` | Explicit params when possible |

### Operators

| Non-canonical | Canonical | Note |
|---------------|-----------|------|
| `x != y` | `x <> y` | Mathematical notation |
| `x && y` | `x and y` | Words over symbols |
| `x \|\| y` | `x or y` | |
| `!x` | `not x` | |
| `x ** 2` | `x ^ 2` | Mathematical power |

### String Operations

| Non-canonical | Canonical | Note |
|---------------|-----------|------|
| `"hello"` | `'hello'` | Single quotes preferred |
| `s.length()` | `#s` | Length operator |
| `s[0]` | `s#0` | Index with `#` |

### Conditionals

| Non-canonical | Canonical | Note |
|---------------|-----------|------|
| `x ? y : z` | `if x then y else z` | Readable form |
| `if (x) { y }` | `if x { y }` | No parens needed |

## Implementation

Normalization hints are emitted during parsing or analysis:

```rust
// In parser or analyzer
fn emit_normalization_hint(original: &str, canonical: &str, reason: &str) {
    eprintln!("hint: prefer `{}` over `{}`", canonical, original);
    eprintln!("      {}", reason);
}
```

### Hint Format

```
hint: prefer `'123' as int` over `int('123')`
      postfix 'as' reads more naturally: value as type
```

### Configuration

Users can control hint behavior:

```wasp
@hints off        // disable all hints
@hints once       // show each hint only once per session
@hints always     // show every time (default)
```

## Adding New Normalizations

1. Identify the non-canonical pattern
2. Define the canonical form
3. Add detection in parser/analyzer
4. Emit hint with clear explanation
5. Document in this file

## Rationale

- **Consistency**: One way makes code easier to read across projects
- **Learning**: Repeated hints build muscle memory
- **Flexibility**: We still accept all forms - hints are suggestions, not errors
- **Gradual**: Users learn the canonical forms over time through gentle nudges
