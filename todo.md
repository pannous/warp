# TODO

The supervisor should remove the DONE elements after a while. 

- DONE: Unbounded Int promotes to bignum on overflow, and `law::lean` exports Warp Int as Lean's unbounded `Int`; `square(3037000500)` is `9223372037000250000` and `law square(x) >= 0` is provable. Explicit `as i64` values still wrap and need a future `BitVec 64` proof model. See notes/laws.md.
- `i<n {…}` without spaces lexes `<n` as a tag (`while i<n {i++}` → garbage); with spaces it works (seen writing lib/uniscript.wasp). TODO That is quite dangerous. How do other HTML languages handle that? 
- `global g = read("f")` → "undefined variable: read"; `global g = "abc"` used in a function → "cannot extract a numeric value"; `const` works.
- A variable first assigned a one-character literal (`s="x"`) is a codepoint variable; `s = s + "ab"` is then a type error.
- DONE: Uniscript: two suffix controls on one character (`<:mirror red A>`) are not expressible; nested tags are not supported. (effect words stack now; nested tags still open)
- tests/test_wasm.rs `test_string_concat_wasm` passes now but is still `#[ignore]`.
- `use x from 1.10` / `use x >= 1.10` read 1.10 as the float 1.1 (only `version 1.10` and literals with two dots keep their text): write `from 1.10.0` or `from version 1.10`.
- DONE: uniscript has no git tag v0.2.0 yet (only the stale v0.1.0): `use uniscript version 0.2.0` works through the default branch's declared version only. (released as v1.0.0 instead, tagged)

- DONE: tests/test_method_words.rs test_library_words_refuse_what_they_cannot_do expects upper("é") to fail with "non ascii text", but the merged text-runtime branch (src/wasm_emitter/text_unicode.rs) now maps Latin/Greek/Cyrillic case, so upper("é") = "É". Decide: drop that expectation or keep the ASCII-only refusal. (2026-10-01 branch consolidation)
- DONE: A runtime ratio has no text form: `y=2.5; y as string`, `str(y)` and now `"x" + y` give "-9223372036854775807" (list_join only knows texts, ints, characters). (fix-sugar 2026-10-02)
- DONE: `x=10; x = floor(x/2)` → WASM validation failure "expected i64, found f64": analyzer::infer_type takes floor's kind from the libm FFI signature (Float) while the builtin floor emits an Int. (fix-sugar 2026-10-02)
- `count of []` parses as `count (of[])` (a subscript of `of`); `count ()` returns the symbol `count`. (fix-sugar 2026-10-02)
- `xs.insert 4 at 0`, `insert 4 at start of xs`, `x is 100 times [0]` (tests/test_lists.rs ignored tests) are not parsed yet. (fix-sugar 2026-10-02)
- DONE: `import floor from "m"; x=10.0; x=floor(2.5)` fails WASM validation (expected i64, found f64): reassigning a float variable from an imported libm call. The builtin (non-imported) floor/ceil/round reassignments work since fix-globals. (2026-10-02)
- DONE: `print` handles literal numbers and texts only: `print(x)` of a runtime value and `print("c")` (a one-character text parses as a codepoint) are "literal numbers and text only so far" errors. (2026-10-02)
- `print` of a list, a float or a map has no runtime text yet ("print of a List has no runtime text yet"), like `xs as string`; a runtime serializer is undecided (tests/test_cast_to_string.rs). (fix-print 2026-10-02)
- `x=1; x+=sqrt(2)` gives the f64 2.414… while `x=1; x=sqrt(2)` keeps the exact real √2: compound assignment skips the exact-real lowering. (fix-print 2026-10-02)
- A statement sequence as the left operand of `or`/`and` at the top level is wrong: `(x=0; x) or 5` gives 0, `(x=0; x) and 5` gives 5; assigned (`z=(x=0; x) or 5`) it is right. `a ≈ b` with a call operand lowers to such a sequence (`f() ≈ 1 or …`). (impl-surface 2026-10-03)
- `print first [10, 5]` and `print(upper "a")` print nothing and give ø: print of a prefix word call; `print first([10, 5])` works. (impl-surface 2026-10-03)
- `x = reduce xs (a b)->a+b` is "functions are not first-class values yet" / "cannot extract a numeric value": the lambda after a prefix word does not stay inside the assigned value; `reduce xs (a b)->a+b` alone works. (impl-surface 2026-10-03)
- `double := it*2; double 4` gives 4: `double` is a type word, so the definition is shadowed silently instead of a loud clash. (impl-surface 2026-10-03)
- `l=π; d=abs(l-3.14); d <= 1e-9*abs(l)` is "l is a float where an exact Int is expected"; inline (`abs(l-3.14) <= 1e-9*abs(l)`) works. (impl-surface 2026-10-03)
- int square(x) = x*x; square(4) + square 3.1; // Automatic return type casting doesn't work.
- def square(x) : float = x*x //This syntax does not work at all. 
- def square(x) as float = x*x //This syntax does not work at all. 

ages = {alice: 30, bob: 25}
for name, age in ages:
  print name,"is",age  // » Error("undefined variable: print")


// Nested block comments are supported
/*
outer comment
/* inner comment */
still in outer
*/  Currently fails. 

result = compute(a, b)  // process inputs      hint prefer a//b over a // b  That makes absolutely zero sense. It should be a warning only if there is a number before and after. 



// Comments attach to the next element as metadata
// This documents the greeting variable
greeting: "Hello"
print(meta of greeting)



// Exception handling
try {
    risky_operation()
} catch error {
    print "Error: " + error.message
} finally {
    cleanup()
}
Error("`try` needs an `else`: `try X else Y`") wtf no



ceo_name = company.employees[2].name // In the data_structure example of the web demo 
Error("undefined function: employees at 32:20")   It's not a function. It's an attribute, and it's not missing. 


» Error("undefined variable: else") WHERE (line number)??  in test_ffi_extended  WHERE??