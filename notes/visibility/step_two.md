# Visibility step two (card untyped-word-privacy), WIP

Step one (on main): `public` marks an untyped library word importers may call; 104 such words marked.
Step two, waiting for a seed that knows `public` (built after step one landed):

1. Mark public the held-back words: os, file, math, and the words the compiler itself reaches
   (map_values, remove_prefix, remove_suffix, run, and the first-pass held ones; measure.py lists the called
   untyped words, report.txt its last run: 525 of 1864 untyped, 179 called).
2. The refusal: word_visibility.parked.warp, an extension. private_calls_refused is registered as passes_before
   modules_used; in extensions/extensions.warp add `use "word_visibility.warp"` and `word_visibility_extension`
   after conversions_extension. A call of an untyped, unmarked library word from a program is refused.

Decision: an untyped (or `any`) library word is private unless marked public; typed words are visible; export
means a host wasm export only.
