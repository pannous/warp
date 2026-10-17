# Self-hosting readiness (survey 2026-10-17, card self-survey)
Question (user): do we have all the parts for the compiler in warp, or do we first need macros so the code doesn't
bloat? Plan: notes/self_hosting.md. Probes: probes/self_hosting/readiness/ (release warp 617af7a75, WARP_NO_WINDOW=1).

## Verdict
- No macro system first. The bloat in a warp pass comes from three things, and warp already has half of each:
  1. the child rebuild every pass repeats (map_children): a shared `bottom_up(n, f)` removes it, but a function
     parameter cannot be passed on yet (card forward-function);
  2. building code by nested `key(…)` / `bracketed(…)` calls: quasi-quote splicing removes it, `data($a + 1)` with
     `$a` spliced (card data-splice). Code patterns with holes (`data($a * 1)`) and `replace … with …` already
     exist; the splice is the missing half of the same feature, and it is all a Lisp macro would give a pass;
  3. kind dispatch by if-chains: kind patterns in `match` exist but fail on parsed nodes (card parsed-kinds).
- Rewrite rules are already there: `program replace data(pattern) with replacement` walks children first and binds
  the holes. With splicing a desugaring pass is a list of rules (planned/deletes.warp: 4 lines for 34 Rust lines).
- Real macros (code that defines code at compile time) would add little on top: passes are tree → tree functions,
  not families of similar definitions. Revisit only if the emitter's runtime functions turn out repetitive.
- The minimal set before the port, ranked:

| # | Card | What | Gain | Size |
|---|------|------|------|------|
| 1 | forward-function | pass a function parameter on (`xs.map(f)`, `x => f(x)`, `walk(x, f)` in a lambda) | shared walk, -4 lines per pass | M (closure capture of function values) |
| 2 | data-splice | `$x` in data(…) templates and `replace … with data(…)`, rest holes `$xs...` | code built as written, -40 % lines | M (switch.rs code holes already bind; build the template with the holes replaced) |
| 3 | data-pattern-sealed | data(…) patterns and templates untouched by the word passes | any phrase (`times`, number words) usable in patterns | S (blocks::seal_quoted exists) |
| 4 | parsed-kinds | parsed `{…}` is a list/block with map and for; `match n { symbol => … }` on parsed nodes; `.bracket` ø on non-groups | dispatch without try/catch workarounds | S-M |
| 5 | built-key-print, bracketed-empty | built keys print their op; `bracketed([], "(")` | twins compare printed text | S each |

Not blocking, card each: unit-hole-name, export-name-clash, parse-typedef, parse-const-fold, parser-quadratic,
quote-parens (note added: brackets in code patterns), carries-line (Meta: parse() drops marks and positions, which
mutation.rs's `x!` marks and every diagnostic need).

## Inventory
| Need (part) | Probe | Today |
|-------------|-------|-------|
| code patterns with holes, `replace` (passes) | node_match.warp | works: `data($a + 0)`, `data($f($x, $x))`, replace at any depth |
| sum types, exhaustive match (analyzer, emitter) | sum_types.warp | works: `enum Shape { Circle(r), Dot, Rect(w, h) }` |
| text-key maps, membership, keys (scopes) | sets_maps.warp | works; no `set()` word, a map of yes is the set |
| classes with mutable fields, globals, try/catch, error values | (inline checks) | work |
| passes as values, closures over locals, `make_adder` | closures_pass_list.warp | work; forwarding a *parameter* fails (forward-function) |
| generic tree walk | tree_walk.warp | gap: forward-function |
| build nodes: `key`, `symbol`, `bracketed` | node_build.warp | build right, print wrong (built-key-print) |
| quasi-quote splicing | quasi_quote.warp | gap: data-splice, data-pattern-sealed |
| node kinds on parse() results | parsed_kinds.warp | partial: `is key/symbol/list` work, `{…}` none (parsed-kinds) |
| parse() as Node | parse_word.warp | works for plain code; classes fail (parse-typedef) |
| modules across files, shared tables | modules.warp | works (`use "path.warp"`) |
| byte output, read back | byte_buffer.warp | works (`write_bytes`, `read_bytes`, a list of ints) |
| 1 MB module byte by byte | byte_buffer_1mb.warp | 1.2 s: fine for now, a packed buffer later |
| float bits (f64.const) | float_bits.warp | works: `float_bits`, `bits_float` |
| LEB128 unsigned and signed | leb128.warp | works (`bit_and`, `bit_or`, `>>` arithmetic; ints unbounded, so i64 wrap needs masks) |
| recursion depth | recursion_depth.warp | 100 000 frames fine |
| scan 120 KB into tokens + keyword map | scan_100k.warp | 0.13 s |
| parse 100 KB, walk 46 001 nodes twice | walk_speed.warp | walks < 0.2 s; parse() 3.4 s = the Rust parser, quadratic (parser-quadratic); parser.warp `tree` ~0.6 s + ~2.2 s compiling parser.warp |

## Bloat test: ports
Code lines without comments and blanks (Rust also without `use`). `ports/` runs today, `ports/planned/` assumes cards 1-4.

| Rust pass | Rust | warp today | warp planned | today's workarounds |
|-----------|------|-----------|--------------|---------------------|
| words/number_words.rs | 61 (28 table) | 14 | 9 | `{…}` not rebuilt (parsed-kinds) |
| words/times_phrases.rs | 43 | 26 | 12 | try/catch `bracket_of`, nodes built by hand |
| mutation.rs lower_deletes + nodes.rs written_entry | 34 | 16 | 4 | `+1` undone by hand (quote-parens) |
| control/switch.rs core (switch, cases_of, no_case, if_then) | 109 | 26 | 10 | subset: no alternatives, no value word, no pattern tests |
| shared walk (planned/walk.warp) | – | – | 2 | written once |
| total | 247 | 82 (0.33×) | 37 (0.15×) | |

Repeated boilerplate in today's ports, and what removes it:
- 3-4 lines per pass: rebuild key / list / leave atom (forward-function + walk.warp).
- `bracketed([symbol("for"), counter, symbol("in"), key(0, "..", n), body], "")` per built form (data-splice:
  `data(for $counter in 0..$n $body)`).
- `try { n.bracket } catch e { "" }` + `n is list or …` group tests (parsed-kinds).
- Not boilerplate but missing: hints and positions (Meta) — Rust passes call `normalize::hint` at the node; warp
  passes need the position on the node (carries-line).

## Surprises met on the way (all carded)
- A hole `$m` reads as the unit metre in the arm; the word then vanishes ("undefined function: picked").
- `node_count` as a user word clashes with a runtime export.
- parse("…g(x - 1)…") of a constant text, given to a user word, is a compile error "undefined function: g".
- The Rust parser takes 3.4 s for 2000 one-line definitions (0.9 s for 1000).
