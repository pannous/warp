done
# slash-notation: `print "abc" matches /a.c/`

Feasible only where a regex is expected; everywhere else a `/` already means something:
- `/tmp`, `/a.c/` where no operand stands before it is a path literal (notes/shell_words.md), and `/a.c/` is a valid
  directory path, so a bare `x = /a.c/` must stay a path;
- `//` is a line comment (the empty regex `//` can't be written), `/*` a block comment;
- `a /2/ c` divides twice.

Done, contextually: the right operand of `matches` (the infix of lib/regex.warp, `use regex`) reads `/…/` as the
regex's text, `\/` a slash in it (src/warp_parser/definitions/operators.rs `slashed_regex`). `//` and `/*` stay
comments, a `/` without its closing one on the line parses as before. No flags (`/x/i`): Rust's regex knows `(?i)`,
JS RegExp does not, so lib/regex has none yet. Calls (`first_match(s, /…/)`, `find_all`) still take quoted texts: that
would need the parser to know which parameters take a regex.

Tests: new modules::test_std_regex::a_slashed_regex_after_matches; test_std_regex:: 4/4, parser:: 316/316, regex 5/5,
path 43/43, comment 63/63. matches 18/19 and samples 50/52 fail only where they fail on origin/main in this container
(test_uniscript; the GPU samples gpu_visualizer_warp, warp_compiled_wgsl_shader, which have no `matches`; no GPU
adapter). `cargo clippy --lib --bins --tests -- -D warnings` clean.

Try: `warp eval 'use regex; print "abc" matches /a.c/'` → yes
