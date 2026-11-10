# Where aliases live (card alias-mechanism, decision alias-mechanism)

lib/aliases.warp is warp's one alias table, plain warp data, one entry per canonical word:

```warp
add { operator: "+", symbol: Op.Add, aliases: [plus] }
modulo { operator: "%", symbol: Op.Mod, aliases: [mod], infix: yes }
"else if" { aliases: [elif, elsif, elseif] }
```

- The entry's key is the canonical form, the one format and print show. An entry keyed by an operator (`"<="`, `"->"`)
  is that operator.
- `operator` / `prefix-operator`: the operator the word names; `symbol`: the operation (Op of core/node.warp).
- `aliases`: the other spellings. A symbol alias spells the operator (`≤` is `<=`, `**` is `^`), a word alias the word
  (`size` is count, `elif` is `else if`).
- `infix: yes`: the word and its word aliases are also written between operands (`7 mod 2`, `1 upto 5`), but name a
  block written right after them (`div {…}` is the html tag).
- Keywords, operator words and constants are quoted (`"and"`, `"yes"`, `"null"`): written bare, the parser already
  reads them as what they are.
- A library keeps the aliases of its own words (lib/math.warp: `alias sin = sine`, `operator ∧ := and`).

## Who reads it

- core/parsing/parser.warp `alias_spellings`, the first time a parse starts (its own parse sees the parser's own
  operators only): symbol aliases join `symbol_operators` (ordered longest first, so `**` wins over `*`), infix words
  join `word_operators`, word aliases of a keyword take its form in `keyword_forms` (`elif`, `has`, `true`, `null`).
  `canonical_word(spelling)` gives the entry's word.
- core/analysis/builtin_signatures.warp `unaliased`: a call's library word is its canonical word, then the prelude's
  `alias keys => map_keys` lines (public name → implementation, not synonyms).

## Next

- Notes: every alias used gets the got-it note "warp says <word>" with the "I meant: <word>" fix; first the compiler's own sources switch to the canonical words, so the stage programs stay
  quiet. Later per-alias data (whether it warns, whether it shows as canonical), maybe as @meta tags.
- More aliases from the wasp list (`!= : unequal, ≢ …`) and warp-layers' list, each checked against the parser: a
  word alias written infix reserves the word.
- alias-network (Later card): a class giving `plus` answers `+`, one giving `add` answers `.plus`.
- math.warp's `operator ∧ := and` lines become entries of math.warp in the same form.

R's and PHP's iteration functions live in lib/R.warp and lib/PHP.warp.
