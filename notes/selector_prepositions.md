# Selector prepositions: of, in, from (card of-in-chain, 2026-10-15)

`of` selects a part: `a of b` is `b.a`, right associative (`length of words of t` is `t.words.length`).
`in` and `from` select the same way when a selector word stands before them and the program does not bind that word
(a variable, a function, a loop variable `for words in xs`):

```warp
characters in "hello"            // ['h' 'e' 'l' 'l' 'o'], "hello".chars
length of characters in "hello"  // 5, length of ("hello".chars)
words from "a b"                 // ["a" "b"]
print words in "ab cd"           // the walk: prints ab, then cd
2 in [1 2 3]                     // yes: anywhere else `in` is the membership test
```

- Selector words: `chars` (alias `characters`), `words`, `lines`, `keys`, `values`
  (src/lowering/collections/of_lookups.rs SELECTOR_WORDS). `letters` waits for warp-interviewer: alphabetic only, or
  every character.
- Implemented in the `of` pass (of_lookups.rs): `in`/`from` after a selector word is one more preposition of
  `looked_up`, so the chain resolves from the right like `of`.
- `print <selector> in t` walks `t.<selector>` (`for part in t.words {print part}`), so printing a selection prints
  each part, as the older walk `print chars in t` did.
- Why right associative: English nests a selecting *of* to the right, "the length of the names of the people"
  (wiki/operator.md "Right associative operators").
