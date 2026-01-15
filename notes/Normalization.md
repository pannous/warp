# Normalization

Wasp accepts many syntactic forms for the same operation, but each has a **preferred canonical form**.
When users write non-canonical forms, we gently educate them about the preferred syntax.

## Philosophy

> "There should be one obvious way to do it."

We embrace syntactic flexibility for input (making the language approachable), but guide users toward consistency.
The normalization hints are shown every time a non-canonical form is used - repetition builds habit.

## Configurable Style

The canonical forms are configurable via the `Style` struct in `src/normalize.rs`.
To change which form is preferred, modify the `Style::default()` implementation.

For example, to prefer `def f(x) {...}` over `f(x) := ...`:

```rust
impl Default for Style {
    fn default() -> Self {
        Self {
            function_def: FunctionStyle::Def,  // Change from ColonEquals to Def
            // ... other defaults
        }
    }
}
```

## Current Canonical Forms

### Type Casting

| Non-canonical | Canonical | Style Option |
|---------------|-----------|--------------|
| `int('123')` | `'123' as int` | `CastStyle::AsOperator` |
| `str(42)` | `42 as string` | |
| `String(x)` | `x as string` | `prefer_string_over_str: true` |
| `float(x)` | `x as float` | |
| `char(65)` | `65 as char` | |

### Variable Assignment

| Non-canonical | Canonical | Style Option |
|---------------|-----------|--------------|
| `let x = 5` | `x := 5` | `VarStyle::ColonEquals` |
| `var x = 5` | `x := 5` | |

### Function Definition

| Non-canonical | Canonical | Style Option |
|---------------|-----------|--------------|
| `function f(x) { x*2 }` | `f(x) := x*2` | `FunctionStyle::ColonEquals` |
| `def f(x): x*2` | `f(x) := x*2` | |
| `fn f(x) = x*2` | `f(x) := x*2` | |
| `fun f(x) = x*2` | `f(x) := x*2` | |

### Logical Operators

| Non-canonical | Canonical | Style Option |
|---------------|-----------|--------------|
| `x && y` | `x and y` | `LogicalStyle::Words` |
| `x \|\| y` | `x or y` | |
| `!x` | `not x` | |

### Power Operator

| Non-canonical | Canonical | Style Option |
|---------------|-----------|--------------|
| `x ** 2` | `x ^ 2` | `PowerStyle::Caret` |

### String Quotes

| Non-canonical | Canonical | Style Option |
|---------------|-----------|--------------|
| `"hello"` | `'hello'` | `QuoteStyle::Single` |

### Conditionals

| Non-canonical | Canonical | Style Option |
|---------------|-----------|--------------|
| `x ? y : z` | `if x then y else z` | `ConditionalStyle::IfThenElse` |

### Indexing

| Non-canonical | Canonical | Style Option |
|---------------|-----------|--------------|
| `s[0]` | `s#0` | `IndexStyle::Hash` |
| `s.length()` | `#s` | |

## Hint Modes

Users can control hint behavior:

```rust
use warp::normalize::{set_hint_mode, HintMode};

set_hint_mode(HintMode::Off);     // disable all hints
set_hint_mode(HintMode::Once);    // show each hint only once per session
set_hint_mode(HintMode::Always);  // show every time (default)
```

Future: `@hints off/once/always` pragma in source code.

## Hint Output Format

Hints are emitted to stderr with colored output:

```
hint: prefer `x and y` over `x && y`
      word operators are more readable
```

## Implementation

Hints are emitted at parse time or analysis time when non-canonical forms are detected:

- **Parser (`wasp_parser.rs`)**: Detects operator forms (`&&`, `||`, `!`, `**`, `?`, `"..."`)
- **Analyzer (`analyzer.rs`)**: Detects function keyword styles (`def`, `fn`, `fun`, `function`)
- **Emitter (`wasm_gc_emitter.rs`)**: Detects type constructor calls (`int(x)`, `str(x)`)

## Adding New Normalizations

1. Add the style enum variant to `src/normalize.rs`
2. Add the hint function to `hints` module
3. Call the hint function at the appropriate detection point
4. Update this documentation
5. Test with `cargo test -- --nocapture` to see hints

## Rationale

- **Consistency**: One way makes code easier to read across projects
- **Learning**: Repeated hints build muscle memory
- **Flexibility**: We still accept all forms - hints are suggestions, not errors
- **Configurable**: Teams can choose their preferred canonical forms
- **Gradual**: Users learn the canonical forms over time through gentle nudges
