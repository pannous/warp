# ≈ approximately (cards approximately, approximately-all; D8)

`a ≈ b` (also `⋍`, `circa`, `approximately`) holds for any two values:

| operands | alike when |
|---|---|
| two numbers | \|a-b\| ≤ tolerance·\|a\| or ≤ tolerance·\|b\| (tolerance 1e-9, or the program's `tolerance = …`, D8) |
| a bool and anything | same truthiness: `yes ≈ 2`, `no ≈ ""` |
| texts, characters | equal after text_fold: lower case, accents dropped (`"hí" ≈ "HI"`) |
| lists, `{…}` objects, instances | same shape, entries ≈ entry by entry (objects in any order, like ==); nested too |
| anything else | `==` |

## How
- library_words.rs lowers `a ≈ b` to the call `values_similar(a, b, tolerance)`.
- wasm_emitter/similarity.rs: two plain numbers (static types Int/Float, not bool) compare inline through
  `numbers_similar(f64, f64, f64)`; any other operands go to the runtime function `values_similar(anyref, anyref, f64)`.
  values_similar and its text_fold table (~25 KB of data) are emitted only when such an operand shows up (a rerun via
  discovered_needs), so numeric programs stay small.
- text_fold is text_lower's emitter with CaseMapping::Fold: a letter whose canonical decomposition adds only combining
  marks maps to its base letter's lower case. Hangul syllables stay themselves. No full case folding: `ß` stays `ß`.

## `~` looser than `≈` (card g_YHSM, P211, P212)
- `a ~ b` (also `~~`) is Op::Rough: the same rules with `rough_tolerance` (default 0.01), and texts are trimmed of
  whitespace, control characters and ASCII punctuation at both ends before folding (text_rough_trim):
  `"Hello!" ~ "hello"`. Lists, objects and instances compare entry by entry with `~` again.
- library_words.rs SIMILARITY_LEVELS: operator, call (values_similar, values_rough), tolerance variable, default;
  similarity.rs emits both runtime functions from one body (`rough` adds text_of + text_rough_trim before text_fold).
- A class method `approximately(o)` is `≈`, `similar(o)` is `~` (class_methods.rs OPERATOR_METHODS); a class defining
  only one has it serve both operators (INTERCHANGEABLE_OPERATORS); with neither, the field-by-field rule.
