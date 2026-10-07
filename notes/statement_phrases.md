# Statement phrases from wiki/thread.md (cards wiki-thread, after-precedence)

## `and` between statements
`sleep 1s and print "second"` runs `sleep 1s`, then `print "second"`: the same as `sleep 1s; print "second"`.
Logical `and` would skip the print, since `sleep` gives ø.

The `and` sequences when both sides are statements:
- **left side:** a braceless call at statement level (`sleep 1s`, `f 3`). The parser flag `in_command` marks its arguments.
- **right side:** a word followed by an argument (`print "second"`, `f 5`). This is `and_starts_statement` in
  wasp_parser/lookahead.rs.

The expression loop stops before such an `and` (`continue_expr`), and the statement list takes it as a `;`
(`parse_list_with_separators`).

Anything else stays logical:
- `x and print "x"`: a value on the left.
- `f(3) and f(5) > 9`: a parenthesised call.
- `a and b or c`: a continuing word after `and`.

## `after C return V`
`five = after n == 5 return v` parses as `five = after·return(n == 5, v)`, the marker call `wasp_parser::AFTER_MARKER`,
like `try X else Y`. The condition runs up to the `return`.

`after` starts this phrase only when a `return` follows on the same statement outside brackets. Without one,
`after tested: body` stays the call listener of lowering/variable_signals.rs.

lowering/go_blocks.rs `after_parts` turns the marker (and the older flat form) into `go { while not (C) { sleep(1) }; V }`.

## Number words (P184)
`one plus two` is 3: lowering/number_words.rs turns the words zero…twenty and thirty…ninety into Ints, each with a
"prefer 1 over one" hint. A word the program names (variable, parameter, function, also C-style `int one()`) stays
the program's: the pass runs after declarations::lower_spaced_definitions so `soft_keywords::program_names` sees those.
Keys (`{one: 1}`) and members (`x.one`) stay words. Not done: compounds (`twenty one`, `one hundred`).
