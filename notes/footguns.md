# Footguns catalogue: how it is maintained

- The catalogue is wiki/Footguns.md (capital F). The checkout is case-insensitive (`core.ignorecase=true`):
  `git add footguns.md` silently stages nothing for the untracked file; use the real name.
- `tests/welcoming/test_footguns.rs`: passing tests = Solved entries; `#[ignore = "next"]` = NOT YET entries with a
  clear answer. When a NOT YET bug gets fixed, un-ignore its test and move the entry to Solved.
- `warp eval` and `is!` use the same `wasm_emitter::eval`, so CLI output is representative.

## Not in the wiki yet

### Assigning a main-level variable from a function
- **Elsewhere:** Python makes `n += 1` inside a function a new local and fails with UnboundLocalError (or silently
  shadows with `n = 5`); JavaScript/Ruby blocks silently mutate the outer variable; PHP sees no outer variables at all.
- **Warp:** a function reads main-level variables by value (captured where it is defined, see Closures). Changing one
  needs the `global` declaration (a State effect, wiki/effects.md): `global n=0; def f(x){n+=1;x}`. Without it,
  `n=0; def f(x){n+=1;x}` is an error that educates:
  `n is a main-level variable: declare it `global n` to change it from a function, or use a new local name; fix: global n`.
  The error applies when the function reads or updates the name before (or without) binding it: `n += 1`, `n++`,
  `xs#i = v`, `y = n; n = y+1`. A body whose first mention is a fresh `name = value` not reading it (`primes = []`)
  is ambiguous and asks (topic `local-or-global`, analyzer::resolve_main_variable_assignments): "a new local of f"
  (default, explicit form `let n = …`) or "the main-level n" (`global n`, which turns main's first `n = …` into the
  global declaration). Unanswered it warns and takes the local, as Python does (samples/lists/sieve_idiomatic.warp relies on
  this); `use strict` makes it an error. `let`/`var n = …`, a parameter of the same name, or a local whose name main does
  not use, is the function's own without a question.
- Tests: tests/welcoming/test_welcoming_globals.rs.

## Uniscript names HTML and LaTeX define differently (P198)
- **Elsewhere:** HTML entities and LaTeX commands share many names with different glyphs: `&circ;` is the modifier
  letter ˆ but `\circ` is the ring operator ∘; HTML swaps `varepsilon`/`varphi` against LaTeX; `&cdot;` is the letter ċ.
- **Warp/uniscript:** the LaTeX reading wins (`\:asymp` ≍, `\:circ` ∘, `\:cdot` ⋅, `\:varepsilon` ε), except for Latin
  letters with a diacritic, which take the HTML one (`\:ocirc` ô, `\:oslash` ø; also `\:imath` ı, `\:jmath` ȷ). The letter rule means letters with
  diacritics, not math names that happen to be letters (exceptions cdot, varepsilon, varphi). A Unicode name ranks above both (`\:tilde` ~, `\:breve` ˘).
  In uniscript since 131b627 (after v1.0.4); warp reads it once its pin is raised past 1.0.4 (src/text/uniscript_entities.tsv,
  tests/parser/test_entity_table.rs).
- Generated, not hand-written: `python3 probes/entities_index/html_latex.py ~/dev/uniscript` (latex.warp keeps the
  LaTeX readings HTML wins over as commented `// name: …` lines).

36 names differ:

| name | HTML | LaTeX | P198 picks | in effect |
|---|---|---|---|---|
| acute | ´ (Sk) | ◌́ (Mn) | LaTeX | ◌́ (Mn) |
| ast | * (Po) | ∗ (Sm) | LaTeX | ∗ (Sm) |
| asymp | ≈ (Sm) | ≍ (Sm) | LaTeX | ≍ (Sm) |
| barwedge | ⌅ (So) | ⊼ (Sm) | LaTeX | ⊼ (Sm) |
| blacktriangleleft | ◂ (So) | ◀ (So) | LaTeX | ◀ (So) |
| blacktriangleright | ▸ (So) | ▶ (So) | LaTeX | ▶ (So) |
| boxbox | ⧉ (Sm) | ⧈ (Sm) | LaTeX | ⧈ (Sm) |
| breve | ˘ (Sk) | ◌̆ (Mn) | LaTeX | ˘ (Sk), the Unicode name wins |
| cdot | ċ (Ll) | ⋅ (Sm) | LaTeX (exception: a math name) | ⋅ (Sm) |
| check | ✓ (So) | ◌̌ (Mn) | LaTeX | ◌̌ (Mn) |
| circ | ˆ (Lm) | ∘ (Sm) | LaTeX | ∘ (Sm) |
| diamondsuit | ♦ (So) | ♢ (So) | LaTeX | ♢ (So) |
| dot | ˙ (Sk) | ◌̇ (Mn) | LaTeX | ◌̇ (Mn) |
| doublebarwedge | ⌆ (So) | ⩞ (Sm) | LaTeX | ⩞ (Sm) |
| fltns | ▱ (So) | ⏥ (So) | LaTeX | ⏥ (So) |
| grave | ` (Sk) | ◌̀ (Mn) | LaTeX | ◌̀ (Mn) |
| Gt | ≫ (Sm) | ⪢ (Sm) | LaTeX | ⪢ (Sm) |
| heartsuit | ♥ (So) | ♡ (So) | LaTeX | ♡ (So) |
| imath | ı (Ll) | 𝚤 (Ll) | HTML | ı (Ll) |
| jmath | ȷ (Ll) | 𝚥 (Ll) | HTML | ȷ (Ll) |
| Lt | ≪ (Sm) | ⪡ (Sm) | LaTeX | ⪡ (Sm) |
| nsubset | ⊂⃒ (Sm,Mn) | ⊄ (Sm) | LaTeX | ⊄ (Sm) |
| nsupset | ⊃⃒ (Sm,Mn) | ⊅ (Sm) | LaTeX | ⊅ (Sm) |
| ocirc | ô (Ll) | ◌̊ (Mn) | HTML | ô (Ll) |
| oslash | ø (Ll) | ⊘ (Sm) | HTML | ø (Ll) |
| perp | ⊥ (Sm) | ⟂ (Sm) | LaTeX | ⟂ (Sm) |
| rightsquigarrow | ↝ (So) | ⇝ (So) | LaTeX | ⇝ (So) |
| star | ☆ (So) | ⋆ (Sm) | LaTeX | ⋆ (Sm) |
| strns | ¯ (Sk) | ⏤ (So) | LaTeX | ⏤ (So) |
| tilde | ˜ (Sk) | ◌̃ (Mn) | LaTeX | ~ (Sm), the Unicode name wins |
| triangleleft | ◃ (So) | ◁ (Sm) | LaTeX | ◁ (Sm) |
| triangleright | ▹ (So) | ▷ (Sm) | LaTeX | ▷ (Sm) |
| varepsilon | ϵ (Ll) | ε (Ll) | LaTeX (exception: a math name) | ε (Ll) |
| varphi | ϕ (Ll) | φ (Ll) | LaTeX (exception: a math name) | φ (Ll) |
| Vee | ⋁ (Sm) | ⩔ (Sm) | LaTeX | ⩔ (Sm) |
| Wedge | ⋀ (Sm) | ⩓ (Sm) | LaTeX | ⩓ (Sm) |

The two-letter codes in parentheses are Unicode General Categories, the property Unicode assigns to every code point:
Sk   │ Symbol, modifier
So   │ Symbol, other
Sm   │ Symbol, math
Po   │ Punctuation,
Mn   │ Mark, nonspacing


## Array covariance (TypeScript's `Array<string>` passed as `Array<string | number>`)
Not a hole in warp: lists are values, so a callee that widens the element type (`xs: list`,
`xs: list of (text or number)`) appends to its own copy; the caller's `names: texts` stays `["hi"]` (also for a typed
class field, a `const` list and a list a closure reads; probes/variance/widening_*.warp). Declared list element types
are checked like scalars, at compile time where known, at the store otherwise (notes/typed_lists.md).

# Tiny footguns
I can't think of a situation where these are problems, but it's a little surprise to newcomers:
~/dev/angles/warp/ warp "type(.0)"    
» int
~/dev/angles/warp/ warp ".0 is float"     
» no
~/dev/angles/warp/ warp "'A' is codepoint"
» yes
~/dev/angles/warp/ warp "'A' is text"     
» yes
