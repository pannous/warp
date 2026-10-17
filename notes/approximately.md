# ≈ approximately (cards approximately, approximately-all; D8)

`a ≈ b` (also `⋍`, `circa`, `approximately`) holds for any two values:

| operands | alike when |
|---|---|
| two numbers | \|a-b\| ≤ tolerance·\|a\| or ≤ tolerance·\|b\| (tolerance 1e-9, or the program's `tolerance = …`, D8), or \|a-b\| ≤ 1e-12 near zero (approx-zero: `sin(3.141592653589793) ≈ 0`) |
| a bool and anything | same truthiness: `yes ≈ 2`, `no ≈ ""` |
| texts, characters | equal after text_fold: lower case, accents dropped (`"hí" ≈ "HI"`) |
| lists, `{…}` objects, instances | same shape, entries ≈ entry by entry (objects in any order, like ==); nested too |
| anything else | `==` |

## How
- library_words.rs lowers `a ≈ b` to lib/prelude.warp's `alike(a, b, tolerance, rough)` (card lists-maps): `{…}` maps
  key by key in any order, other lists item by item, pairs and instances (a class name paired with its fields, as an
  `any` value) part by part, each with alike again; any other two values go to the runtime function
  `values_similar(anyref, anyref, f64)` (bool, uncertain, numbers, texts, else values_equal).
- wasm_emitter/similarity.rs: a call of alike with two plain numbers (static types Int/Float, not bool) compiles inline
  to `numbers_similar(f64, f64, f64)`. Any other call loads alike as a compiler word (late_calls.rs compiler_word: a
  second compile when the program did not name it), so a numeric program stays ~3 KB instead of carrying alike, the
  map runtime and text_fold's table (~25 KB of data).
  folding.rs never specialises alike's calls (a variant would hide the call from the emitter).
- text_fold is text_lower's emitter with CaseMapping::Fold: a letter whose canonical decomposition adds only combining
  marks maps to its base letter's lower case. Hangul syllables stay themselves. No full case folding: `ß` stays `ß`.

## `~` looser than `≈` (P211, P212)
- `a ~ b` (also `~~`) is Op::Rough: the same rules with `rough_tolerance` (default 0.01), and texts are trimmed of
  whitespace, control characters and ASCII punctuation at both ends before folding (text_rough_trim):
  `"Hello!" ~ "hello"`. Lists, objects and instances compare entry by entry with `~` again.
- library_words.rs SIMILARITY_LEVELS: operator, runtime function (values_similar, values_rough), tolerance variable,
  default; alike's `rough` picks the function; similarity.rs emits both from one body (`rough` adds text_of + text_rough_trim before text_fold).
- A class method `approximately(o)` is `≈`, `similar(o)` is `~` (class_methods.rs OPERATOR_METHODS); a class defining
  only one has it serve both operators (INTERCHANGEABLE_OPERATORS); with neither, the field-by-field rule.
