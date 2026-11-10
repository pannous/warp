# Dot call plan: `b.a` ≡ `a(b)` (decision fields-are-functions, card fields-are-functions)

Investigation only (warp-dot-call-plan, 2026-11-09). Probes run against main 5e6fdf72f3.

## Today

`x.w` is read in four places, each knowing a slice of the rule:

| where | what it does |
|---|---|
| extensions/sugar/method_calls.warp (late sugar) | `x.f(a…)` → `f(x, a…)` (no field-first: `o = {a: 10, f := it*a}; o.f(3)` fails "a function value in ."); module words `math.gcd(…)`; `m.remove(k)` |
| extensions/sugar/methods.warp | `xs.push/add/append/insert/pop` in place; `x.json` → json_text(x); `x.upper!`; spaced arguments `t.replace "a" with: "b"` |
| extensions/sugar/reflection.warp | ~25 dot rules: `f.name .params .parameters .body .doc .effects .signature .return_type .definition .full`, `C.fields .methods .functions`, `p.fields .methods`, `m.functions .attributes .classes .operators`, most twinned with a call rule `fields($c)` … |
| extensions/of_lookups.warp | `a of b` → `b.a`, but `type of x` special-cased to `type(x)` because `x.type` fails |
| extensions/property_accessors.warp (187 lines) | `canvas.width := …` getters/setters as a table of bodies pasted in place |
| extensions/class_methods/methods.warp | methods lifted to functions; a method named like a library word routed by static type (`sum·Bag`, dispatcher `sum·method`) |
| passes mirroring calls | arity_overloads.warp:143,173, prelude_words.warp:619,716, lambdas.warp:81,108,238,364-370: each has a `Key(receiver, Op.Dot, call)` case = its call case with argument offset 1 |
| analyzer.warp:203,599 | `x.length/count/size…` is Int, every other `x.w` Data |
| emitter.warp:1347-1384 (Dot branch) | 1. receiver of known class + struct field → `struct.get` (hot path); 2. `counting_properties` → field_or_word(length); 3. `type(x).name`; 4. `member_word` (runtime word of one parameter, via `library_words`) with `holds_entry`/`keeps_word`/`declares_field` exceptions; 5. `byte_count_words`; 6. Time fields; 7. `method_function` (program function taking x alone, arity variant); 8. library word taking x with defaults; 9. `field_of` → has_field ? map_get : trap "no field w" |
| control_flow.warp:175 field_or_word | THE run-time rule already: on a Node receiver `has_field(x, w) ? map_get(x, w) : word(x)`; on a typed receiver `word(x)` directly |
| runtime fields.warp / maps.warp map_get, has_field | own parts hard-coded beside entries: `.line .column` (meta), `.bracket .separator` (lists), `.name` (symbol, function, entry), `.key .value .op` (entries), `.message` (errors), swizzles `.xy` |

Probes (scratchpad, WARP_NO_WINDOW=1):
- works: `sq(x):=x*x; 3.sq` 9, `m.a.sq`, `3.f(4)`, `"ab".upper`, `xs.sum`, `xs.sort`, `"a".ord`, `x.round`, `m={keys:1}; m.keys` 1,
  `g(m):=m.size; g({size:9})` 9 (field first at run time), `class P{type:int}; P(3).type` 3, `m.a.b.c`.
- fails, though `w(x)` works: `3.type` / `xs.type` / `P(3).type` ("no field type", the card), `16.sqrt`, `x.abs`, `x.floor`,
  `x.text`, `g = x => x*2; 3.g`, `o.f(3)` (function-valued field), `P(3).name`.
- The failures are exactly the words the emitter compiles as its own forms (`type`, `floor`, magnitude_words `abs sqrt cbrt`,
  `text_words`, `cast_words`, `ord`, `chr`, `number`, bit casts) or local closures: the Dot branch asks `module.functions`
  and `library_words`, never the call path itself.

## The rule

```
x.w        = x's own w if it has one (struct field, map entry, module member, own part), else w(x)
x.w(a…)    = x's own w called with a… if it has one (method, function-valued field), else w(x, a…)
x.w = v    = x's own field w set if it has one, else w(x, v) where a setter of that arity takes x, else a new entry (map) / extra field (instance, warned)
```
"Own" = most specific overload: one rule, no word lists. Left-associative, so `a.b.c` is `c(b(a))` step by step, nothing special.

**Where**: one place, the emitter's Dot branch (core/emitter/emitter.warp), because only it knows x's static kind:
1. static own field (known class struct field, Time field, map literal `holds_entry`) → read it (struct.get unchanged).
2. `w` callable on x (`callable(body, w, [x])`, below) → `field_or_word(value, w, r => (w r))`: a typed receiver calls
   directly, a Node receiver tests `has_field` first at run time.
3. else `field_of` (has_field ? map_get : "no field w").

**Compiling `(w r)` with an evaluated receiver**: bind the receiver's local under a hidden name (`·receiver` in
body.names, as `fresh_local` already holds it) and compile the group `(w ·receiver)` through `expression_code`: every
call form is reached the same way a written `w(x)` is (program functions, arity variants, typed overloads, class
dispatchers `speak·method`, runtime words, emitter forms like `type`, closures in locals). No second call path.

**`callable`**: `w` is a local holding a function, `method_function(body, w, [x])`, a `library_words` entry taking x,
or an emitter call form. Today the emitter forms are a chain of `if bracket_of(n) == "(" and #n == 2 and n#1 == symbol("type")`
lines (emitter.warp ~1186-1200); step 2 turns them into one map `call_forms = {type: …, floor: …, ord: …}` whose keys
answer `callable` and whose values the group branch dispatches on: the chain gets shorter, not longer.

**Run-time fallback without a table**: `w` is always spelled in the source, so the function is chosen at compile time;
only "own field or function" is decided at run time (the has_field branch field_or_word already emits). A function
overloaded by type dispatches through its existing per-name dispatcher (class methods `w·method`, field getters
`field_getter_prefix + w`). No name → function table at run time.

**Hot path**: typed receiver + struct field: `struct.get`, untouched. Typed non-Node receiver + function: a direct
call, no test. Node receiver: has_field + branch, the same instructions field_of emits today (it just traps in the
else arm instead of calling). Zero cost for code that works today.

## Plan

| step | change | core lines (est.) |
|---|---|---|
| 1 | Dot branch: steps 2,4,5,7,8 (counting_properties, member_word+keeps_word, byte_count_words, method_function, library word) collapse into `callable` + `field_or_word` with the hidden-receiver call; `type(x).name` special (line 1355) goes (`.name` of a type value is its own part) | −30 +14, emitter.warp |
| 2 | emitter call forms as one `call_forms` map (the `(w x)` chain), keyed for `callable` | −15 +12, emitter.warp |
| 3 | analyzer.warp:203,599: kind of `x.w` = kind of `(w x)` when x has no static own w; `counting_words` list goes | −2 +3 |
| 4 | `x.w(a…)` field-first: emitter Dot-call: x's own method / function field (static class, or run time `has_field` → closure call), else `(w x a…)`. method_calls.warp's two general rules go; `o.f(3)` works | −2 rules, +8 emitter |
| 5 | runtime own parts as ordinary words: `line(n) column(n) bracket(n) separator(n) key(e) value(e) op(e) name(x) message(e)`; map_get/has_field keep only entries, instances and swizzles. node-part-clash (`.key` of `key = 3`) falls out of field-first | fields.warp/maps.warp −25 +9 |
| 6 | sugar cleanup (extensions, no core): `type of x` special in of_lookups, `x.json` rule (needs `json(x)` word), reflection.warp's dot twins of call rules (`$c.fields`/`fields($c)` …: keep the call rule, drop the dot one: ~10 rules) | −15 lines |
| 7 | assignment `x.w = v` → setter `w(x, v)` when x has no own w; property_accessors.warp (187 lines) becomes overloads `width(c: Canvas)` / `width(c: Canvas, w)` once canvas/window/display are class instances | later, own card |
| 8 | optional: parser/first pass canonicalizes `x.w(a…)` to `w(x, a…)` marked with the attribute `receiver` (with_attribute exists); the mirror `Key(receiver, Op.Dot, call)` cases in arity_overloads, prelude_words, lambdas go (~7 branches, ~15 lines) | −15 +4, needs OK |

Steps 1-3 are the card (≈ −47 +29 core lines, net smaller). 4-5 follow-ups, 6 rides along, 7-8 own cards.

## What it deletes
counting_properties, member_word, keeps_word, byte_count_words in the Dot branch, the library-word Dot step, the
`type(x).name` special, analyzer counting_words for dots, method_calls.warp's general rules, of_lookups' `type of`
special, ~10 reflection dot twins, the hard-coded node parts in map_get/has_field, later property_accessors.warp and
the Dot mirrors in three passes.

## Risks
- Typos become calls: `p.nmae` on a Node receiver still says "no field nmae" (no function nmae); but `p.text` on a map
  without `text` now gives the shown text instead of an error. That is the decision; error quality only drops where a
  global function of that name exists.
- Data shadows words on Node receivers: JSON `{"type": "x", "count": 3}` answers `.type` and `.count` with its own data
  (field first), not the word; today `keeps_word` lets some words (`m.keys` of an unknown map) win over an entry. The
  rule removes keeps_word; a program wanting the word writes `keys(m)`.
- Self-compile: the compiler's own `x.w` on untyped Nodes, where w also names a compiler function, now compiles a call
  arm (bigger warp.wasm, maybe slower compile); measure with scripts/vitals.py; mitigation: type the receiver.
- In-place list methods (`xs.push`) stay sugar rules before the dot rule (they mean `xs += [v]`, not push(xs, v)).
- Module members vs reflection: `math.sqrt` is math's own member, `time.functions` = functions(time): the same rule, but
  module_named must run before it (it does: sugar).

## Status (branch fields-are-functions, 2026-11-09)
Answers (decision dot-call answers): own methods first, then fields, then functions; o.f(3) calls the field's
function; local closures by dot; `x.w = v` setter by arity; step 8 after 1-5.
- Steps 1+2 landed as one: emitter `member_read` (own field → call w(x) via `called_on` → field_of). The receiver is
  held in a named scratch local (`scratch_name`) and `w(x)` compiles through the call path; the emitter's one-value
  forms are `is_own_word`/`own_word_code`, reached by `w(x)` and `x.w` alike (a table of lambdas was not needed).
  Optional returns (`TypedCode?`) broke the seed ("a ? where a list … is needed"): predicate + code instead.
- Map protocol: the self-compile died on `entries.keys` of the compiler's library_words, a dictionary holding the
  entry "keys". The words declared with a map first (`takes_map_first`: keys, values, get …, taking the map alone)
  are the map's own methods and win over its entries; `m["keys"]` reads the entry. `p = {has: 2}; p.has` stays 2:
  has takes two values. When protocol-default-implementation declares `protocol map`, its method set replaces
  typed_lines as the source.
- Local closures (step 4 part): in the lambdas pass (dotted_lowered), where function_values knows which variables
  hold functions; only a local or parameter, and only where no object of the program declares a field of that name
  (`field_names`, Op.Colon keys except `: function`): `task.value` beside a local `value` stays Task's field.
- definitions.warp is_function_block: an object whose function entries bind their own `it` is data. `o.f(3)` with
  `f := it*a` still fails (on main too): the entry lambda reads o, which its own initializer binds → card
  object-computed-field.
- Step 3 (analyzer) skipped: the analyzer's Data for `x.w` is the safe superset; the emitter's TypedCode carries the
  call's kind. Step 6: of_lookups' `type of` is a parser quirk (`type of` reads as a declaration head), not a dot
  workaround: stays. Step 5 (node parts as words) dropped: global words `name value key line …` cost more
  than the ~15 lines of map_get they save.
- Step 7 (decision setters-declared, extensions only): a class's one-parameter method is its setter
  (class_methods setter_calls, no getter needed); a function whose first parameter is typed with x's class is one too
  where x is made as that class and declares no field w (sugar rule in methods.warp + rule word class_setter). Maps
  and untyped values keep the entry write: every two-parameter function would else hijack `m.merge = 3`. A rule
  pattern with `=` needs parentheses, `rewrite ($x.$w = $v) => …`: unbracketed, `=>` binds tighter.
- Step 8 parked: card canonical-call-rewrite (Later). 38 sugar rules in 10 files match `x.w(…)`; it waits for a
  rewrite-engine feature matching a call and its dot form in one pattern.

## Open questions for the user
1. `o.f(3)` with `f` a function-valued field: the field's function called with 3 (recommended, object-functions) or
   `f(o, 3)`?
2. Data shadows words: `json.count` of `{"count": 3}` is 3 (field first, recommended: the decision's rule) and `keys(m)`
   is the way to the word; drop keeps_word's exceptions (`m.keys` of an unknown map holding "keys")?
3. Local closures: `g = x => x*2; 3.g` is 6 (recommended: `w` resolved like a call `g(3)`)?
4. Setter by assignment: `x.w = v` on a value without field w calls a two-parameter `w(x, v)` when one takes x
   (property-accessors' rule generalized), else adds the entry (map) / extra field (instance, warned) — OK?
5. Step 8 (dot calls canonical as `w(x, …)` from the first pass, removing the Dot mirrors in three passes): worth a core
   pass change now, or after steps 1-5?
