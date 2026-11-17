# wap's data reader: a spec inspired by wasp

Target: `wap/core/parse/reader.wap` (today a 7-line stub that calls warp's `parse`). Read first:
notes/first_principles.md, notes/backbone.md, notes/wap.md "The pipeline".

Citations. `W` = ~/dev/angles/wasp/source/Wasp.cpp, `P` = WaspParsers.cpp, `C` = CharUtils.cpp,
`L` = LiteralParser.cpp, `K` = Keywords.cpp, `O` = OperatorAnalyzer.cpp, `A` = Angle.h, `N` = Node.cpp,
`T` = ~/dev/angles/wasp/test/tests.cpp. (test_parser.h and test_node.h only declare the tests; the bodies are in
tests.cpp.) `warp:` = /Users/me/dev/angles/warp/core/parsing/.

The goal is wasp's general idea of reading nodes, not a copy of its code. Wasp reads data and language in one pass,
with options and keyword branches mixed in. Wap keeps three steps strictly apart:

1. **The data reader** (this spec). It turns any text into Node and knows nothing about the language.
2. **Spelling rules** (shape rules). These find the language's shapes in the Node tree: keywords, `if … then`,
   `def`, `for … in`, operator precedence, units, `true` …
3. **The analyzer**: declarations, binding, typing.

Each wasp behaviour below is tagged **[1]** (it stays in the reader) or **[2]** (it moves to the spelling rules).

## 1. What the reader is and isn't

- **It is:** a pure notation reader. It reads a superset of JSON5-like data: atoms, keys, brackets, separators,
  comments, `@meta`, and juxtaposition (`a b c`). The same reader reads data files and programs ("data and code are
  one notation").
- **Output:** one `Node` (warp/core/node.warp:5) per text. It uses `Number`, `Char`, `Text`, `Symbol`,
  `Key(left, Colon, right)`, `List(items, bracket, separator)`, `Meta(node, data)`, `Empty` and `Error`.
  - The reader never makes `True` or `False`. Those come from the step-2 alias table (see §4, item D1).
  - The reader never makes `Type` or `Data`.
- **It is not** a language parser, an operator-precedence parser or a keyword recogniser. It never uses a word list,
  the operator table or parser modes (wasp's ParserOptions, A:107-122). Only character classes and spacing tell one
  item from another.
- **Size goal:** a few hundred lines. warp's reader is 3,919 lines, about 1,700 of them keyword branches.

## 2. The lexical layer

### 2.1 Character classes (the only thing that tells an operator from a word) [1]
Wasp decides by character class, without any word table (C:9-16, C:91-112):

- **Name characters:** letters, `_` and `$` (C:10); any code point above 128 that is not an operator glyph (C:14);
  digits, but not at the start.
- **Operator glyphs:** ASCII punctuation other than brackets, quotes, `;` and `,` (C:111); arrows 2190-21F3 and
  math 2200-2319 (C:100-101); superscripts (W:146, C:99); `¬` (C:102). `∞`, `∅` and `⅓` are exceptions: name or
  number characters (C:96-98).
- **Brackets:** `( ) [ ] { }`, plus the other Unicode pairs (W:104-108).
- **Quotes:** `" ' « “ ‘ \`` (W:1204-1209).
- **Blanks:** space and tab. Newline is not a blank: it separates (W:844).

For wap, "is a letter" should be a general Unicode test, not ranges by hand. warp already does this: a code point
whose upper case differs from its lower case, plus the CJK and Hangul ranges (warp:lexer.warp:271).

- A **word** is a run of name characters, read as `Symbol`.
- An **operator** is a run of operator glyphs, also read as `Symbol`.
  - Wasp joins a glyph only with a repeat of itself or with `=` (`++ ** == += <=`), plus `=>` (W:883-889). That is a
    hidden table.
  - Wap reads the maximal run of glyphs, Swift-style. Step 2 splits a run the operator table doesn't know (see §6,
    item Q5).

### 2.2 Whitespace, newlines and indentation [1]

- **A blank** is the space separator (§3.2). Blanks after an opening bracket or before a closing one mean nothing
  (`{\n}` is `{}`, W:1058-1070).
- **No blank between two items glues them** (§3.3). This is the one piece of spacing the reader records.
- **Indentation** (W:248-292, W:492-503). A deeper line opens a block and a shallower line closes it.
  - Wasp turns the newline into the control characters INDENT and DEDENT (0x0F and 0x0E). These then act as one more
    bracket pair (W:234-235, W:1189, P:244-257). This is the elegant part: an indented block is just a bracket.
  - Indent unit: tabs, or 2 spaces per level (W:253). Mixing tabs and spaces is an error (W:269, W:276).
  - Indentation inside an open bracket opens no block: `a{\n  b,c}` is `a{b,c}` (W:496, T:3817-3822).
  - Empty lines are ignored, so they never change the level (W:288-289, T:2111-2131).

### 2.3 Comments [1, kept as Meta]

- `// …` runs to the end of the line (W:819-823).
  - In wap, `//` glued after `:` is no comment (`http://`), as warp does it (warp:lexer.warp:113).
- `/* … */` and `/# … #/` are block comments (W:824-827). In wap they nest, like warp (warp:lexer.warp:121-139).
  Wasp's closer is loose: `*#` and `##` also close (W:779-784).
- `#` starts a comment only when a blank or the line start comes before it **and** a blank comes after it
  (W:803-807, W:1233-1240).
  - `#!` and `##` are comments too in warp (warp:lexer.warp:277-282).
  - A `#` glued to what follows is an operator, the count: `#[0 1 2 3]` (T:3293). See wiki/comments.md.
  - **[2]** warp's `#use`/`#include` lines as directives (warp:lexer.warp:281) are language. They move out.
- **Difference:** wasp throws comments away (W:837-850). Wap keeps them as `Meta` on the next item, or on the item
  that ends the line. wiki/data.md promises `settings#1.comment`.
- A comment counts as a blank for gluing: `y/* c */=0` is `y =0`.

### 2.4 Numbers [1]

Wasp's `numbero` (W:557-626) reads digits, an optional `.` and fraction (`3.` is 3.0, T:2415), an exponent `e±n`
(W:587-599) and hex `0x…` (W:542-554, T:1319-1323); octal is only a todo (W:563-566). A number is a float if it has
`.` or a negative exponent (W:601). Wap adds digit separators `1_000` (warp:parser.warp:3777-3781) and warp's guard
that `1..3` and `1.max` don't eat the point (warp:parser.warp:3756).

Things that move out:
- **[2]** A number glued to a name, `3x` or `3ms`, becomes `*(3, x)` inside wasp's reader (W:615-624). Wap reads it
  as the glued run `G(3 ms)`. Units and implicit multiplication are rules ("units are values").
- **[2]** The `+1` warning (W:567-568) is a style lint for a later stage.
- **Signs.** Wasp reads `-` or `+` before a digit as part of the number when a blank, an operator or nothing comes
  before it (W:926-928, P:349-352). So `[a -1]` is two items, `a-1` and `a - 1` are a subtraction, and `1 - -3` works
  (T:477-478).
  - The backbone wants "no separate token rule": a minus is a prefix operator on a literal, folded later.
  - See §6, item Q4 for the choice.

### 2.5 Texts and characters [1]

- **Quotes.**
  - Wasp: `"…"`, `'…'`, `«…»`, `“…”`, `‘…’` and backticks (W:1204-1209, closers at C:70-83).
  - warp today: `"`, `'` and `«` (warp:parser.warp:3797).
  - `«…»` holds quotes as they are.
- **Escapes.**
  - Wasp has a table (`\" \\ \/ \b \f \n \r \t`, L:7-30) but never uses it: `quote()` returns the raw substring
    (W:668-679). The full escape reader is commented out (W:682-747).
  - Wap reads escapes in the reader: `\n \t \r \e \\ \" \' \x41 \u{1F600}`, as warp does
    (warp:parser.warp:3862-3919). An unknown escape is refused, not kept as is.
- **One character:** a text of one code point is a `Char`. wiki/char.md:11 says `'a' == "a"`, and warp does this
  too (warp:parser.warp:3812).
- **[2]** Templates: wasp splits `` `x $y ${e}` `` into chunks inside the reader (W:634-666). In wap the reader
  keeps the raw `Text`. A step-2 rule splits it and calls the reader on each hole (§6, item Q8).

### 2.6 Words with `$` and `@` [1]

- `$cond` is one name, `Symbol("$cond")`, because `$` is a name character (C:10). Patterns bind `$names`
  (decision pattern-dollar-holes). Wasp also had `$`/`%`/`@` name options (P:10-14, W:1247-1253). Drop them.
- `@name` and `@name(args)` attach to the item that follows, as `Meta(item, data)` (P:16-33, T:293-318). That is
  notation, not language: "`@meta` is always a meta node".

## 3. The structure layer

### 3.1 Brackets: three kinds of list [1]

`getType` (W:1086-1105) maps each bracket family to one kind:
- `{` and its fullwidth variants are objects;
- `(` is a group;
- `[` is a pattern;
- `<` is generics, but only under a mode.

In data all three are just lists (wiki/data.md: `{1 2 3} [1 2 3] (1 2 3)` all denote a list). Wap reads
`List(items, Curly|Square|Round, separator)`. The reader never flattens a bracket. `(c)` stays a one-item round list,
and step 2 drops grouping parentheses.

- **[2]** `<…>` as tags or generics (P:40-80, W:1176-1182) is language. In the reader `<` and `>` are operator glyphs.
- **[2]** Wasp's HTML `<html>`/`<script>` raw blocks (P:40-63) never belong in the reader.
- **Other bracket pairs** (`‖x‖`, `⟨x⟩`, `⌈x⌉`) are, in wasp, "overloadable grouping operators": a node named by the
  bracket with the body inside (W:1111-1122, W:929-931). This is a good general idea, so keep it. See §6, item Q7
  for the Node shape.
- **Errors:** a stray closer and an unclosed opener are errors (W:1195-1198, W:1261-1262, T:2660, T:726-727).

### 3.2 Separators and the cascade [1]

The separators are:
- blank (space), `,`, `;`, tab (between items of one line), and newline;
- the end of an indented block, which closes like a bracket.

Wasp's best idea: separators have a precedence. A looser separator splits first, and the pieces between it split
by the tighter ones. So `(1 2; 3 4)` is a 2×2 matrix with no extra brackets (wiki/grouping.md).
- Precedence table: newline 2, `;` 3, `,` 4, space 5, where a higher number binds tighter (W:1329-1345).
- `closing()` ends an inner list when a looser separator comes (W:309-331).
- `parseListSeparator` wraps the list so far into a new level when a new, looser separator appears (P:260-298).
- The separator is stored on the list (`Node.separator`, Node.h:147).
- warp: Space 0 < Comma 2 < Semicolon 3 < Tab 4 < Newline 5 (warp:parser.warp:866-873), so warp adds tab as a
  column separator (warp:parser.warp:858-864).
- Wasp bug, don't copy: `0 < c < 0x20` is checked before `'\n'`, so newline and tab both get 1.5 (W:1337-1338).

Wap's order, tightest first: **glue < space < `,` < `;` < tab < newline**.
- A `;` at the end of a line is a newline (warp:parser.warp:850).
- A trailing separator is dropped (`1,2,3,`, W:1066-1068).
- Inside brackets, the list takes the loosest separator used.
- An unbracketed list of one item is the item itself.
- Separator `None` means nothing between the items: a glued run, or fewer than two items.

### 3.3 Juxtaposition and glue [1]

- **Juxtaposition.** `a b c` is a list, `List([a b c], None, Space)`, never a call (W:973-1002, T:2677-2680).
  - Wasp marks the list `groups` if it has no operator and `expression` if it has one (W:983, W:989, W:994), and
    keeps it flat.
  - Wap needs no mark: step 2 sees the operator symbols itself.
- **Glue.** The tightest separator is no blank at all. Wasp uses glue ad hoc:
  - `a{x}` attaches the block to `a`, while `a {x}` is two items (P:92, P:130-135, T:2035-2040, T:2071-2074);
  - a sign before a digit (§2.4);
  - kebab-case names (W:1317-1322).
- In wap, glue is general. Items with no blank between them form one glued run, `List(items, None, None)`:
  - `f(x)` is `G(f (x))`;
  - `a{x:1}` is `G(a {…})`;
  - `xs[1]` is `G(xs [1])`;
  - `a.b` is `G(a . b)`;
  - `3ms` is `G(3 ms)`;
  - `a-b` is `G(a - b)`;
  - `-x` is `G(- x)`.
- That gives step 2 one input for every spacing rule: the minus rule of backbone.md "Minus", `f(x) == f (x)`, kebab
  names and units. The reader decides none of them.

### 3.4 Keys [1 for `:` only]

Wasp's short and long binding (W:1124-1128, P:175-239):
- A glued `:` binds tightly. `a b:c d` is `a (b:c) d` (T:2075-2077).
  - In a glued run, `:` splits the run: `x:40` is `K(x, 40)` and `a-b:2` is `K(G(a - b), 2)`.
- `:` glued left with a blank after it takes the rest of the item, up to the next separator looser than space.
  `a: b c d` is `K(a, S(b c d))` (T:2078-2081).
  - This differs from wasp, where a comma doesn't end the value (W:223, W:316). JSON5 needs `{a: 1, b: 2}` to be two
    keys.
- `:` with blanks on both sides takes the item so far as its key. `a b : c` is `K(S(a b), c)` (P:189, P:228-231,
  T:2082-2084).
- `:` at the line end followed by an indented block makes `K(a, block)` (P:217-222, T:2147-2166).
- `:` with a blank before it and glued after it (`a :b`) stays as the glued run `G(: b)`. Step 2 decides.
- **[2]** All other key-like operators are language: `=`, `:=`, `≔`, `←`, `->`, `=>`, `＝`. Wasp folds them in the
  reader (W:1212-1222, P:328-338), and in "data mode" it reads `=` as `:` (P:201, T:2044-2066). In wap they stay
  `Symbol`s in the phrase, and the operator stage builds `Key(l, Assign, r)` and the others.

### 3.5 Operators and precedence: wasp does NOT apply them in the reader

This matters most. Wasp's reader keeps arithmetic flat:
- `expressione` collects `symbol()`s into one flat list and marks it `expression` (W:973-1002);
- `parseExpression` splices that list into the current one (P:363-396).

Precedence comes later, in the analyzer:
- `groupOperators` (O:60) calls `collectOperators` (O:9-39);
- that sorts the operators found in the flat list by the numeric `precedence()` table (K:578-…);
- then it folds them tightest first, from the right for right-associative and prefix operators (O:96-99).
- A prefix minus is told apart there, as `-…`, when an operator comes before it (O:19-20).

Only three things bind inside wasp's reader: separators, by their own precedence; `:` and `=` keys (§3.4); and `3x`
implicit multiplication (W:615-624).

warp's current reader is the opposite. It is a Pratt parser with binding powers (`expression(p, min_power)`,
warp:parser.warp:1009-1060), so precedence and keywords live in the reader.

Wap follows wasp, and goes further: the reader keeps operators as flat `Symbol`s in phrases, and a later stage
applies the library operator table (backbone.md "Operators"). No precedence, fixity or `:=` exists in the reader.

### 3.6 Behaviours of wasp's reader that move to step 2 (language) or are dropped

| wasp | where | wap |
|---|---|---|
| `true/yes/✔`, `false/no`, `null/nil/ø` word lists turned into True/False/NIL | W:90-96, W:907-911, T:3214-3231 | [2] alias table in lib; the reader gives `Symbol` |
| operator words (`and or not is in of to xor …`) marked as operators | W:137-152, W:913 | [2] operator table |
| `fun`/`def` → `parseDeclaration` with block markers `: := = is be do` | W:975-976, W:1268-1313 | [2] rule |
| `)` then `{` is a declaration, except after `while` and `if` | P:108-115 | [2] rule |
| `import`/`include` read the file inside the reader | P:300-322, P:371-372 | [2]/[3] |
| `while` named in `addSmart` | N:671-674 | [2] |
| URL by a scheme list (`http`, `ftp` …) | W:122, P:179-183 | [1] general: `//` after `:` is no comment, no list |
| `=` as `:` in data mode; `colon_immediate`, `space_brace`, `use_tags` … | A:107-122 | dropped: one reading |
| `lookahead_ambiguity` scans the line for `, : ; =` | W:948-965 | not needed: separators cascade |
| `flat()` and `addSmart` collapse singletons by heuristics | N:1106-1137, N:636-690 | dropped: no flattening |
| `setField` copies values differently in debug builds | W:1012-1022 | bug, never copy |
| `a {x}` warning "ambiguous block" | P:94-100 | [2] lint (the reader records glue) |

## 4. Ideas worth keeping (with the principle each serves)

1. **Separator precedence and the cascade** (W:309-331, W:1329-1345, P:260-298): n-dimensional data with no
   brackets. This is one general rule (principle 2).
2. **Flat phrases, precedence later** (W:973-1002, O:60): the reader stays language-free (principle 11), and the
   operator table lives in lib (principle 17).
3. **Operators by character class** (C:91-112): no word table in core (principle 18).
4. **Indentation as a bracket pair**, INDENT and DEDENT (W:234-235, W:492-503): blocks by braces or by indent are one
   form.
5. **Spacing is information** (wiki/significant-whitespace.md): wasp uses it for signs, keys and attaching. Wap
   generalises it to glued runs.
6. **Short and long colon** (W:1124-1128): `a b:c d` and `a: b c d` read the way people expect.
7. **Bracket families map to three kinds** (W:1086-1105), and any other pair is a grouping operator (W:1111-1122).
8. **`@meta` attaches to the next item** (P:16-33).
9. **Forgiving borders**: trailing commas, `{\n}` (W:1058-1070).
10. **Indentation inside brackets is no block** (W:496, T:3817).

Never copy: the keyword lists and parser modes (§3.6); raw strings with no escapes (W:668-679); the
separator-precedence bug (W:1337); debug-dependent results (W:1012); `3x` as `*` inside the reader (W:615); the
`\` "escape anything" in `parseNode` (W:1149-1153).

## 5. Examples (wasp test → wap reader output)

Shorthand: `S(…)` = `List(…, None, Space)`; `G(…)` = `List(…, None, None)`, a glued run; `C(…)` / `Q(…)` / `R(…)`
= Curly / Square / Round lists, with the separator noted as `,` `;` `⏎` when not space; `K(a, b)` =
`Key(a, Colon, b)`; `M(n, d)` = `Meta(n, d)`; bare words are `Symbol`s.

A "≠ wasp" note marks a deliberate difference.

| # | input | wap reader output | wasp test |
|---|---|---|---|
| 1 | `1` | `1` | T:2398-2399 |
| 2 | `3.` | `3.0` | T:2415 |
| 3 | `0xFF` | `255` | T:1321 |
| 4 | `'hi'` | `"hi"` | T:2419-2423 |
| 5 | `` (empty) | `Empty` | T:3228 |
| 6 | `yes` | `yes` (≠ wasp True; step 2 aliases) | T:3223 |
| 7 | `null` | `null` (≠ wasp NIL; step 2) | T:3227 |
| 8 | `1 2 3` | `S(1 2 3)` | T:2677-2680, T:3181 |
| 9 | `(1,2,3)` | `R,(1 2 3)` | T:3183 |
| 10 | `1;2;3` | `List(1 2 3, None, Semicolon)` | T:3185 |
| 11 | `[1 2 3]` | `Q(1 2 3)` | T:3187 |
| 12 | `{  }` | `C()` (empty curly list) | T:3279-3280 |
| 13 | `{aa:3}` | `C(K(aa, 3))` (≠ wasp, which flattens to `aa:3`) | T:2400-2405 |
| 14 | `a:'HIO' b:3  d:{}` | `S(K(a,"HIO") K(b,3) K(d,C()))` | T:2629 |
| 15 | `{deep{a:3,b:4,c:{d:'hi'}}}` | `C(G(deep C,(K(a,3) K(b,4) K(c,C(K(d,"hi"))))))` | T:2601-2606 |
| 16 | `a b:c` | `S(a K(b,c))` | T:2075-2077 |
| 17 | `a: b c d` | `K(a, S(b c d))` | T:2078-2081, T:2237 |
| 18 | `a b : c` | `K(S(a b), c)` | T:2082-2084 |
| 19 | `a{x:1}` vs `a:{x:1}` | `G(a C(K(x,1)))` vs `K(a, C(K(x,1)))` | T:2037-2040 |
| 20 | `a b (c)` | `S(a b R(c))`: three items | T:2071-2072 |
| 21 | `a b(c)` | `S(a G(b R(c)))`: two items | T:2073-2074 |
| 22 | `div{ class:'bold' 'text'}` | `G(div C(K(class,"bold") "text"))` | T:2338-2341 |
| 23 | `body(style='blue'){a(link)}` | `G(body R(G(style = "blue")) C(G(a R(link))))` (≠ wasp: `=` stays a symbol) | T:3244-3245 |
| 24 | `x:40;x+1` | `List(K(x,40) G(x + 1), None, Semicolon)` | T:386-388 |
| 25 | `a b; c d` | `List(S(a b) S(c d), None, Semicolon)` | T:3786-3788 |
| 26 | `a b c, d e f` and `a,b,c;d,e,f` | `,(S(a b c) S(d e f))` and `;(,(a b c) ,(d e f))`: same shape | T:3789-3803 |
| 27 | `{ a b , c d ; e f , g h }` | `C;(,(S(a b) S(c d)) ,(S(e f) S(g h)))`; with `⏎` for `;`, same shape, separator Newline | T:3808-3813 |
| 28 | `a{\n  b,c}` | `G(a C,(b c))`, the same as `a{b,c}` | T:3817-3822 |
| 29 | `a⏎  b⏎  c⏎d⏎e` | `⏎(G(a C⏎(b c)) d e)`: the block attaches like `a{b c}` | T:2168-2185, T:3319 |
| 30 | `a:⏎  b⏎  c⏎⏎d⏎e` | `⏎(K(a, C⏎(b c)) d e)`: the empty line changes nothing | T:2111-2131, T:2147-2166 |
| 31 | `blah a b c # note⏎ y/* c */=0 // c` | `⏎(S(blah a b c) S(y G(= 0)))`, comments as Meta (≠ wasp flat `y = 0`) | T:2102-2109 |
| 32 | `1 - -3` / `1- -3` | `S(1 - -3)` / `S(G(1 -) -3)` | T:477-478 |
| 33 | `#[0 1 2 3]` | `G(# Q(0 1 2 3))`: glued `#` is no comment | T:3293 |
| 34 | `@attrib(1) @attrib2(42) tee{a:1}` | `M(G(tee C(K(a,1))), S(G(attrib R(1)) G(attrib2 R(42))))` | T:306-318 |
| 35 | `1 m + 1km` | `S(1 m + G(1 km))`: units are step 2 | T:3980 |
| 36 | `a-b:2 c-d:4 a-b` | `S(K(G(a - b),2) K(G(c - d),4) G(a - b))`: kebab names are step 2 | T:1690-1694 |
| 37 | `` `hello $test` `` | `"hello $test"` (raw; the template is step 2) | T:408 |
| 38 | `]` / `(unclosed` | `Error` | T:2660, T:726-727 |

Rows 26 and 27: wasp's `==` ignores separators, so its tests only pin the shape. Wap keeps the separators, and the
shapes match.

## 6. Decided (user 2026-11-17)

- **Q3 indentation:** an indented block is exactly `{…}`, a curly list with separator Newline, glued to the line
  before it; tabs or a consistent run of spaces, mixing refused.
- **Q4 `-1`:** a sign glued to digits at the start of an item is a negative Number literal; `-a` (sign glued to a
  word or a bracket) is the prefix operator. `a -1` is `[a, -1]`; `a-1` and `a - 1` are infix.
- **Q9 `true`/`false`/`null`:** the reader gives `Symbol`; the alias table decides.
- **Q10 comments:** kept as Meta, on the next item or on the item that ends the line.
- **Q1 `a{x}`:** a head glued to a bracket group is `Key(head, Glue, group)`: `a{x:1}` is `Key(a, Glue, {x:1})`,
  `f(x)` is `Key(f, Glue, (x))`, distinct from `a:{x:1}` (`Key(a, Colon, …)`). Only head + bracket; other glued
  runs stay Q2's list. Confirmed 2026-11-17: a glued run `[a {x:1}]` would look like a two-item list.
- **Q2 glue:** runs without brackets (`3ms`, `a-b`) are a List with separator `None`; no new Separator case.
- **Q5 operator runs:** the reader splits operator characters itself, as wasp: it joins only a repeat or a
  following `=` (`==`, `<=`); `x=-1` reads `=` then `-1`.
- **Q6 `#`:** the reader detects comments; the glued `#` operator (count, `#use`) comes later, in step 2.
- **Q7 other bracket pairs `‖x‖ ⟨x⟩ ⌈x⌉`:** a `Bracket` case Other; details later.
- **Q8 interpolation:** the reader returns the raw Text; a step-2 rule splits the `${…}` holes.
- **Q11 `=` in data:** the reader never turns `=` into `:`; `a=1` reads as `Key(a, Eq, 1)` (confirmed 2026-11-17).
- **Q12 duplicate keys:** the reader marks them with a Meta warning on the curly list; step 3 decides.

## 7. Open questions for the user (wasp and warp differ); recommendation first

