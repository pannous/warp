# Error classes (decision error-classes, card error-types)

```warp
class ParseError: Error                  // or: class ParseError extends Error {}, class ParseError(Error): pass
class TokenError extends ParseError {}
try raise TokenError("bad char") except ParseError as e: e.message   // "bad char"
try 1/0 except ZeroDivisionError: 7      // 7
try 1/0 except IndexError: 1             // the division error passes on
```

- lib/prelude.warp declares `class Error { message: text }`. A class extending a class the program does not declare
  takes the prelude's (inheritance.rs `prelude_parent`); `extends Error` counts as mentioning Error, so the prelude
  slice comes along (declarations.rs mentioned_names).
- Runtime errors have Python's classes, with Python's parents (error_classes.rs RUNTIME_ERROR_CLASSES):
  LookupError > IndexError, KeyError; ArithmeticError > ZeroDivisionError, OverflowError; ValueError, TypeError,
  MemoryError. runtime_errors.rs `runtime_error_class` maps each trap to its class.
- An Error node carries its class as a symbol in its value field (`class_error(text, class)`); `instance_of(e, "C")`
  of an Error compares that symbol.
- `raise C(…)`, `raise C{message: …}`, `raise C` of an error class lower to `raise_as(message, "C")`
  (error_classes.rs raised_instance): the trap detail plus the global trap·class, which `catch`/`except` read back.
  A plain `raise "text"` clears the class.
- `except C as e: Y` parses to the fourth item `e: C` (`caught·error: C` without `as`). lower_try_binding tests
  `instance_of(e, C) or …` over C and every class extending it; any other error is raised on with its class
  (`raise_as(text_form(e), error_class(e))`). `except Error`, bare `except:` and `catch e` catch everything.
  Several excepts chain: `try X except A: a except B: b` is `try (try X except A: a) except B: b`.
- Compile errors with the fix: `except FooError` / `raise FooError(…)` of no declared class →
  "FooError is no error class: declare it with `class FooError: Error`".
- After inheritance a class keeps its parent as the INHERITED_FROM mark (the EXTENDS attribute is gone then, so
  inherit stays idempotent); error_classes.rs reads both.
- parser.warp twins: guarded / guarded_by / caught_parts, and `class X: Parent` in declared_type.

- `x is Animal` holds for a Dog extending Animal at any depth: the type registry keeps each class's parent
  (EXTENDS or INHERITED_FROM), emit_instance_of compares the class and its descendants.
- A class may extend a runtime error's class (`class MyIndexError extends IndexError {}`): inheritance.rs takes in
  `class IndexError extends LookupError {}` from error_classes.rs runtime_class_declaration, as it takes in the
  prelude's Error.
- An error a non-matching `except` passes on keeps its position: the catch keeps trap_position
  (trap·caught·position), `raise_on` puts it back before raising. The position is the statement's, as for any
  runtime error.

Open:
- A raised instance keeps only its message: other fields of an error class do not survive the raise.
- `e is LookupError` of a runtime IndexError reads the type registry, which knows only declared classes: `except`
  sees the runtime class tree, `is` does not yet.
