# Decided questions (user answers)

History of answered decisions, newest first. Grep-only: look up a cited number (P71, D5, #10) or grep a topic
before asking the user; nobody reads it front to back, the code, tests and wiki are the truth (user 2026-10-08:
"If it's really grep-only, then it's perfect. Otherwise, move them to a history file"). References to
"notes/open_decisions.md" + a Decided section, P-, D- or #-numbers mean this file. Open and parked questions:
notes/open_decisions.md.

## Decided 2026-10-10 (user; via warp-supervisor or the Interviewer)
- footgun-divzero (warp-numbers): user: "no idea about the advantages … you decide or we decide later"; the
  Interviewer's default, revisit any time: exact numbers (ints, rationals, decimals) raise divide by zero (`r = try
  1/0; r failed` is yes, as failed-raised), computed IEEE floats give ∞/NaN as IEEE (`sqrt(2)/0` is ∞). The
  extended-rational ±1/0 idea stays open for later; wiki Footguns "NaN and infinity" is corrected to this.
- wiki-full-evaluation (warp-docs): no conflict: `created: now` stays a block until `!` (the spec); "dates are folded
  at compile time" is today's implementation limit (Footguns "Not yet"), not a rule. Implied, not asked.
- let-reassign (warp-types; user, asked 2026-10-10): `let` is fully immutable, `var` is the variable. `let x = 1; x = 2`,
  `x += …`, `x++` and `xs#1 = 5` on a let are all errors with the fix "use var x". Supersedes P159's "changing a let
  works with a note" and the wiki/mutable.md:23 split (value may change, binding fixed); the wiki follows this.
- module-global-assign (warp-sound default, not asked): a program's top-level `tempo = 90` / `note_seconds = 0.25` /
  `canvas_width = 800` sets the standard module's `global` of that name instead of making a shadowing program
  variable; other module words still shadow. Revisit if it surprises the user.
- exact-default (user 2026-10-10, asked after warp-perf measured fraction math as ~90% of finger paint's time plus a
  never-freed number heap): integer division stays exact, `1/3 + 1/3` is `2/3`; decimal literals are floats,
  `.1 + .2 ≈ .3`; `==` on floats gives a warning with the fix "use ≈". Supersedes the 2026-09-28 "decimals exact"
  (`0.1+0.2==0.3`); tests pinning it change meaning by this decision. The ratio heap leak gets fixed regardless
  (ratios as GC values, small ones inline).
- let-mut (user 2026-10-10, via warp-supervisor): `let mut x` means `var x`, with a loud warning that warp writes
  `var`, not the long form.
- runtime-dates (user 2026-10-10): "dates are evaluated at compile time" must not stand in the wiki; dates get a run-time
  representation, `now` reads the clock at run time (card runtime-dates, Now).
- tour-firefox-stall (warp-web): a site whose program runs in a Worker ships task-workers.js (the page creates the
  task Workers; Firefox stalls on new Worker inside a Worker); tests/web/test_site.rs expected lists gain it.
- failed-raised (warp-fixer, card failed-raised): a raised error (1/0, raise, [1]#5) keeps stopping the program; a
  bare `try X` (no else) turns it into the value: `r = try 10/0; if r failed then 1 else 2` is 1, `r = 10/0` stops
  with divide by zero. `try` without `else` is allowed from now on.
- rational-float (warp-numbers): `x: rational = sqrt(2)` is a compile error naming rational ("√(2) is a float where an
  exact rational is expected: declare it `number` to keep the float, or truncate with `as int`"), like a rational
  parameter. `x: real = sqrt(2)` works (card real-holds).
- canvas-zero (warp-sound): `use draw` + `show()` without `canvas(…)` uses a default canvas (the playground pane or
  the window size, else 640×480), never an error. User: "who demanded … errors? We should have defaults." The
  branch's "a 0×0 image is empty" error is undone. General rule (welcoming.md): a missing setting takes a sensible
  default; errors only for ambiguity too dangerous to guess.
- arctan-allow (warp-class): `arctan := arc_tangent` names the function (as `arctan = &arc_tangent`). Partial
  application only with an explicit hole: `inc := add(1, _); inc 5` is 6. Too few arguments without a hole
  (`inc := add 1`) stays the error "add needs 2 arguments", whose fix names `add(1, _)`.
- int-list (warp-numbers, card int-list): a decimal literal stays exact and is int when whole (`type([0.0, 0.0])` is
  list of int, P196b stands); a variable's type joins its literal with every write the analyzer sees, so
  `shown = [0.0, 0.0]; shown[0] = sqrt(2.0)` makes shown a list of float from the start. Runtime conversion only as
  the fallback for writes the analyzer can't see. As implemented.
- type-equal (warp-numbers, cards type-int and type-equal; implied by #30 "Only `is` tests types", per warp-supervisor):
  `is` is the subtype test and `==` between types is exact. `type(0.0) is int`, `type(0) is number` and
  `int is number` are yes; `type(0) == number` is no; `type(2) == type(3)` is yes. Built on the user's rule (cards
  type-value-type, real-equality): a bare type name is the type as a value, not a constructor, so `type(π) == real`,
  `t = type(0); t is int` and `int == int` work like any value; `3 == int` is no with the hint toward `is`.
- people-where (warp-class): no new `with`/`without` sugar after a list beyond what is already implemented (user:
  the words have too many meanings; `with` could as well mean "with something added"), and "if it's already
  implemented, keep it implemented": main's `with` filter from the orm example stays (`name of people with age > 20`,
  tests/control/test_field_of_elements.rs), and warp-class's field-filter `without` (`people without team` = `people
  where not it.team`) is kept. Removing values (`[1 2 3] without 2`) stays undefined.
- P238 (warp-fixer, card generators-function): a plain generator call collects the list: `count_to(3)` is [1 2 3],
  `sum(count_to(4))` is 10. `iter(count_to(3))` or `next` on a variable holding it gives the lazy object; a `for`
  over it runs lazily. As implemented.
- let-comma (warp-types): `let a, b = 3, 4` (also var/const) unpacks: a = 3, b = 4, as the undeclared `a, b = 3, 4`.
  As implemented.
- P239 (warp-class, cards lambda-def, unbound-lambda): in a lambda naming its parameter, `it` is the surrounding
  code's `it`: `scale := [1 2].map(x => x * it); scale 3` is [3 6]; with no outer `it`, `[1 2].map(x => it)` is
  the error "undefined variable: it". As implemented.
- P240 (warp-web, card gpu-auto; revises P214): GPU calculations are imprecise by design (f32), and the user is told
  so. Heavy maps of linear float arrays switch to the GPU automatically from 10× the measured break-even
  (GPU_AUTO_MIN_COUNT), with a one-time run-time notice; `@cpu` (a map, block, function or program) and
  `WARP_GPU=off` (a whole run) keep results exact in f64; `@gpu @cpu` on one map is an error. GPU-vs-CPU tests compare
  within common::gpu_tolerance, @cpu on the reference side. notes/gpu.md "Precision".

## Decided 2026-10-09 (user, as recommended unless quoted)
- warp-ast.wit moves from the repo root to samples/wit/warp-ast.wit; test_wit_parse reads it there (user chose the
  move over deleting it).
- scripts/own-warp.sh stays (the user's checkout deletion is not landed): uniscript build, hosting test, probes and
  notes/agents/common.md still use its private scratch/warp copy. Revisit once callers move elsewhere.
- Fermyon / Akamai Functions (warp-hosting, card fermyon-hosting): the user signs up and puts a personal access token
  in .env as akamai_functions_token for a live test of "☁ Deploy to Fermyon"; visitors paste their own token, kept
  in their browser.
- `if c {1} else 3+1` (user's multiple choice, via the supervisor; card else-print, batch 74): the else takes the
  whole `3+1`, giving 1 or 4, not `(if … else 3)+1`.
- Bee/wasp entity names go into uniscript and are regenerated into src/uniscript_entities.tsv, never added by hand
  in warp (user, via the supervisor).
- P235b amended (the user's own edit of samples/webgpu.warp, `let uv = at.xy / $size;`; Integrator, card
  shader-holes for warp-web): `$name` in a shader block is an explicit hole, as in sql templates: the compiler
  passes that warp value as `values.name` and builds the values map itself. Bare WGSL names still never capture
  warp variables. `$` never occurs in WGSL, so no clash; the uniform layout exists (src/gpu.rs uniform_layout).
  Defaults: the name is resolved where the block is written, the value read at each paint call.
- Hosting login callback (warp-hosting): NOT as recommended: GitHub's callback stays https://lambda.pannous.com/callback;
  the pannous.com server proxies lambda.pannous.com → warp-hosting.pannous.workers.dev (Ferron block + certbot cert,
  edited in pannous-lockdown scripts/levels/rustweb.sh). The hosting Worker is live; secrets come from .env and ~/.keys.
- Units are written glued to their numbers, as a soft hint (user to warp-types, via the supervisor; card
  unit-glue, branch unit-glue-hint 12da9e1d0): `3 km` works with the hint "prefer 3km over 3 km"; samples write
  `3km`.
- Hosting (warp-hosting, card cloud-hosting; each program is its own Cloudflare Worker uploaded by the hosting
  Worker https://warp-hosting.pannous.workers.dev): Q1 the user creates a GitHub OAuth App for "Log in with
  GitHub"; Q2 the user creates a Cloudflare "Edit Cloudflare Workers" API token for uploads; Q3 free Workers plan
  for now (~90 programs, 100k requests/day), Workers for Platforms ($25/month) only near the cap; Q4 programs live
  at warp-<name>.pannous.workers.dev, <name>.warp.pannous.com later (pannous.com's nameservers are mixed);
  Q5 a private Cloudflare OAuth client now for deploying to one's own account, others paste a scoped API token
  kept in their browser.
- Word slices (user to the Interviewer: "list from A to B. List starting from A. List items to B"): with
  xs = [10, 20, 30, 40, 50], `xs from 2 to 4` → [20, 30, 40], `xs starting from 2` → [20, 30, 40, 50],
  `xs up to 2` → [10, 20]; positions count like `xs#1` (first = 1), both ends inclusive. Slicing by position,
  not by value and not building a range.
  Amended the same day (user, via the supervisor: "up to 2 is very confusing when it also contains the number 2"):
  positions are marked, with the hash (`xs from #2 to #4`, `xs up to #2`) or ordinals (`from second to fourth`,
  `up to 2nd`, `nth`). A bare number in a word slice (`xs up to 2`) is a loud error naming `#2` and `second`
  (user's multiple choice, via the supervisor); positions are only `#n` or ordinals.
- Playground CI (user, via the supervisor): pages.yml runs Chrome and Firefox as parallel jobs; workers are not
  required to tour new examples with --firefox before a merge request.
- Standing rule (user, via the supervisor): "always present questions in multiple choice form so I get informed
  either here or via the interviewer": every user question, the Supervisor's included, is an AskUserQuestion popup.
- Hosting of deployed warp programs (user, via the supervisor): "let's start with our own login and let people log
  in with their own button". First our account hosts the programs: users log in with us and get a Deploy button.
  Second, a button deploying to the person's own provider account. Worker warp-hosting researches providers
  (notes/hosting.md), builds both, and sends account, cost and DNS questions here.
- Hosting, third option (user, 2026-10-09, multiple choice: "let's also create a third option"): both Ferron on
  pannous.com (built: warp-lambda, `<name>.lambda.pannous.com`, notes/hosting.md) and Fermyon / Akamai Functions
  ("Both now"). Programs on pannous.com run sandboxed: no C, shell or other runtime, files and SQLite only in their
  own folder, web requests allowed, a locked-down systemd unit each.
- Ranges (user, via warp-class, branch range-descriptor 51cdcb2dd): "We don't need the colon syntax if we have the
  dot-dot syntax": `r: 1..n` is the range itself, same as `r = 1..n`; `..` marks a value, so no uncharged-block
  warning.
- P236 (warp-keywords, card golf-echo): NOT as recommended: `warp run` keeps echoing the final value after the
  script's prints, as today (`for i in 1 to 2 { print i; x = 1 }` → 1, 2, 1).
- P235 (warp-web, card g_oFJc): WGSL is written as a `shader { … }` block, read verbatim with balanced braces, its
  value the shader text; `wgsl { … }` is an alias. `shader{…}` is no longer tagged data (`Shader{…}` still is).
  P235b: a shader does not capture warp variables; inputs stay explicit, `paint(rings, w, h, {frame: frame})` and
  `values.frame` inside.
- Not asked (word choice, warp-class, card g_mnvA): word infix operators are declared as in the user's own line,
  `infix operator divides(d:int, n:int) := n % d == 0` (parameters optional, `left`/`right` otherwise, precedence
  of `+` per P48); `infix divides(d, n) := …` is an alias. samples/orm.warp's `n divides d` in is_prime is reversed
  under that definition and becomes `d divides n`.
- P234 (warp-web, card graphics-names, from the user's TODO "We want elegance, but not black magic"): paint also
  takes a WGSL shader text and renders it on the GPU: `paint(shader, size, size, {frame: frame})`; given pixels it
  shows them as before. gpu_render stays for getting the pixels. Word choice, not asked: `use graphics` is an alias
  of `use draw`.
- P233 (warp-keywords, card field-tolerance): a field typed with a tolerance gives every value that tolerance:
  `class Part{length: m ± 1 mm}`, `Part(5 m).length` → `5.000 ± 0.001m` (a spec attached to each value). Replaces
  the interim "not supported yet" error.
- Undoable default (warp-class, card g_mSEw "a less explicit form"): `form post "/todos" { input{name:"title"}
  button{"add"} }` = `form{ method:"post" action:"/todos" … }`, matching the route `post "/todos" {…}`;
  samples/todo_app.warp. Not built: naming the route function (`form add_todo {…}`) or finding the only matching
  post route by the form's inputs.
- P232 (warp-class, card g_mQ9U): `$x` as "field x of the implicit subject" stays in served routes only
  (`$title` = request.body.title / request.query.title); not in event handlers, `it` contexts or component props.
  `$a` in data literals keeps referencing the enclosing node `a{…}` (references.rs).
- The user's uncommitted main-checkout edits (supervisor; patch scratch/user_edits.patch) land:
  literals.rs: `${}`, `$()`, `\()` and `\{}` interpolation are all fine, no hint (user: "${} $() \() \{} all fine");
  the 2 test_interpolation tests expecting the hint change. orm.warp: `bo.age += 1` writes through without
  `save bo`; the sample test must tolerate the age growing on each run. natural.warp: land the countdown check with
  its stray trailing `,2` removed. extensions.rs: "should not be a copy, but a hard link to some dev folder, and it
  should reflect the new state of this project": the current file has link count 1 and old content (wasp_parser,
  `strings!`); restore warp's content and re-link it with the shared copy.
- P231 (supervisor/warp-web): print rounds quantities with units to a decimal, `mean of [5km, 1.5km, 12km]` prints
  `6.17km`; str() and serialization keep the exact `(37/6)km`; plain numbers keep `7/3`. Replaces the interim
  `(37/6)km`-everywhere build.
- P230 (card effects-value): NOT as recommended: `effects of f` gives symbols `(State IO)`, not texts; needs symbol
  values in the emitter. The interim text-list default and its test edit are undone.
- P228 (warp-fixer, card standalone-std-io): no; programs using tables or JSON don't build stand-alone, the runtime
  stays ~1 MB; a native run notes "host.std_io needs runtime." on the first run only. Wording changed the same day
  (user's own src/main.rs edit, via the supervisor): the note reads "<features> used runtime.", without the
  "(said once until the file or warp changes)" remark.
- P226b (test_logarithm2): `x⌟b` is log base b of x: `100⌟10` → 2, `10⌟100` → 0.5; `x⌟` alone is ln x.
- P229 (card g_gHmE): settled by the user outside the code: "I already changed it to open my editor. Nothing to fix
  here." Double-clicking a .warp file opens the editor; nothing to build.
- Served routes (undoable defaults, supervisor, card served-route): a route that raises on a browser form answers 400
  and re-renders the page with the message; a missing form field gives 400, an unknown id 404.
- `is empty` covers "", [] and {} as well as ø, following wiki/null.md (undoable default, supervisor).
- /api and /rpc JSON give instances as plain objects, `[{"name":"Ann","id":1}]`, not wrapped in their class name
  (undoable default, supervisor, asked by warp-class).
- P225 (warp-worker, test_string_operations): a float joined to text uses its shortest form, as print does:
  `'say ' + 0.` → "say 0"; the old test expecting "say 0." changes.
- P226 (warp-worker, test_logarithm2): the log glyphs come, and they differ (user: "yes add them but they are not the
  same"): `b⌞x` is log base b of x (`10⌞100` → 2); `⌟` is a postfix log (`ℯ⌟` → 1). Its forms with a base: P226b
  (queued).
- P227 (warp-worker, test_emit_cast_tuple): a comma next to `==` without parentheses is an error asking for them
  (user: "insist on braces to avoid errors"): `(2.0, 4) == 2.0, 4` → error, write `(2.0, 4) == (2.0, 4)`.
- Not asked (legacy names / test typos): test_sinus and test_sinus2 use `x % tau` instead of the C++ helper
  `modulo_double` (no alias), test_sinus loses its stray quote; test_recent_random_bugs: legacy lines go to skip!
  or are fixed to warp's meaning (`use math;`, parentheses, √π² = π) and the test is un-ignored.

## Decided 2026-10-08 (user, as recommended unless quoted)
- ORM updates (undoable default, supervisor, from the user's own edit of samples/orm.warp; warp-fixer): assigning a
  field of a stored row (`bo.age += 1`) writes through as an SQL UPDATE; `save bo` is accepted as an explicit write,
  a no-op when the row is already written.
- Every major or semi-major feature ships with a sample in samples/, especially the ORM and the server (user, to the
  supervisor; rule in notes/agents/common.md 2ca368e9f; cards sample-orm, warp-fixer, and sample-server,
  warp-functions).
- P224 (card plus-minus-print, warp-class): `<`, `>`, `<=`, `>=` on a ± interval answer only when certain, like
  Julia's IntervalArithmetic: with `y = 6 ± 1` (5..7) `y < 8` → yes, `y < 4` → no. `≈` stays as P218.
  P224b (user: "we don't want the application to crash" on values read from outside): no run-time crash.
  Where the compiler knows a value is ±, a bare comparison is a compile error asking for one of
  `y certainly < x`, `y possibly < x`, `y.value < x`. Where it can't know (data read at run time), an overlap
  counts as not certain (`certainly`, the condition is no) and a warning is printed once naming the line, with
  the hint to write `certainly`, `possibly` or `.value`.
- `//` is a comment only when followed by a space or the end of the line; otherwise floor division: `x //= 3`,
  `x//=3`, `7//2`, `a //b` divide; `a // b`, `x = 1 // note` are comments (user, emphatic, via the supervisor;
  replaces the 2026-10-03 rule, rewritten in place below). warp-worker implements it.
- P223 (card orm, warp-functions): filters may name an element's fields bare, like SQL: `people where age > 20`
  means `people where it.age > 20` when the list holds a known class with that field (database tables included)
  and no variable of that name is in scope; a variable `age` in scope wins, with a warning that it is also a field.
- With P222 (user: "warp serve can also get an argument; can we just call warp server.warp if the file is
  obviously a server?"; criteria are the supervisor's undoable default): `warp serve [file] [port]`; plain
  `warp app.warp` serves when the program declares a route/get/post handler or a server def (static check),
  printing the URL and the `warp run` escape; markup-only programs don't auto-serve; `warp run` and `warp test`
  never serve.
- ORM (card orm, warp-functions; user in a discussion with the supervisor): plain classes connect to the database
  without inheritance or annotations, they are only registered. Transactions are completely optional (an
  optimization only). Filters work for any warp expression ("magic": SQLite application functions run warp code
  inside queries). Lazy loading: a smart default chosen by us, fine-tuning keywords later. Migrations: start with
  the supervisor's proposal.
- `x /= y` means exactly `x = x / y`: `x = 3; x /= 2` → 3/2, prints 1.5; floor division is `x //= y`; a variable
  declared `x:int` keeps a whole number. Card div-assign, warp-worker switches the floor tests to `//=` (user,
  asked directly by the supervisor).
- P221 (card route-sample, warp-functions; discussed in free form): one line serves both sides. A route such as
  `route "/users/:id:int" { h1{ users#id } }` whose block reads server data (a database, files, secrets) runs on the
  server: the first visit gets finished HTML, later in-app clicks fetch only the data and render in the browser
  (like Next.js/SvelteKit); the compiler splits it, the user writes no JavaScript and nothing secret ships. Routes
  that don't touch server data stay static. The browser never talks to the database itself. User: "both from your
  single line YES!!". The single-page decision (2026-10-07) holds only for the playground web demo; real
  applications can have as many pages and server routes as they want (user).
- P217 (card plus-minus, warp-class): `5 ± 1` is an interval now (user: "I thought they are just interval"):
  worst-case bounds, `(5 ± 1) + (2 ± 1)` → `7 ± 2`, functions map the endpoints (`sqrt(4 ± 1)` → √3..√5).
  Gaussian propagation (Measurements.jl) comes later with an explicit form such as `5 ± 1σ` (user chose
  "interval now, Gaussian later").
- P218 `x ≈ r ± 1` holds when |x - r| ≤ 1, the tolerance as written (user: no preference; the default stands).
- P219 ± values print with 2 significant digits of the ± part, the value rounded to the same place: `7.0 ± 1.4`.
- P220 one meaning of ±: `1950 ± 50 AD` is the same ± value (an interval of years), not a separate units span.
- P222 (card route-sample, warp-functions): the CGI mode of `warp serve` is retired; `warp serve app.warp 8080`
  serves the page program (its routes, `server def` functions as POST /rpc/f), the production twin of `warp dev`.
- From P200b (not asked; card shared-lists-typed, warp-class): a typed list read from a field shares too:
  `p = {xs: [1, 2]}; ys = p.xs; p.xs#1 = 7; ys#1` → 7. The fast unboxed copy is kept only where the compiler proves
  that neither alias is written afterwards; otherwise the list is shared, never a silent snapshot.
- P216 (warp-types): named arguments run as written, left to right (like Python): with `global i = 0;
  g() := { i = i*10+1; i }; h() := { i = i*10+2; i }; f(a, b) := a*100 + b`, `f(b=h(), a=g())` runs h first
  (b=2), then g (a=21) → 2102; the corpus pin 112 (parameter order) changes. Applies to constructors too
  (card named-constructor-args).
- P214 (card gpu-vectors, warp-web; revises P118): big list math moves to the GPU automatically only where the result
  is identical to the CPU's: Int lists fitting i32, with an overflow flag (the CPU redoes it on overflow). Float lists
  go to the GPU only when the program allows f32: `@gpu …` or a `float32[n]` list. `sum(int[10^7] .* 3)` → GPU;
  `sum(float[10^7] .* 0.1)` → CPU. User: "for @gpu give a warning or hint if the GPU does not apply" (it then runs on
  the CPU). The GPU covers the dotted element-wise operators (`xs .* 2`, `xs .+ ys`).
  Threshold (user): offloading usually pays off from about 10⁵–10⁶ elements, clearly from 10⁶–10⁷; one cheap op
  like `xs .* 3` may never pay off unless several ops are fused or the data stays on the GPU. Err on the CPU side.
  Once the feature works, measure where and when to enable it and set safety bounds from real measurements.
- P215 (card shared-lists, warp-class): with shared lists, element types are checked both ways, per P203: where the
  alias is visible a covariant assignment is a compile error (`xs: [Circle] = [c]; ys: [Shape] = xs` → error unless ys
  is read-only); otherwise each write (add/insert/`xs#i = v`) checks the declared element type and fails loudly at
  run time.
- P213 (card loop-value, warp-worker): a loop ending in `print` gives ø, because print gives ø (#18) and a loop's
  value is its last body value (P55); no pass-count special case. `for f in [foe1, friend1]: print it` → ø
  (tests/control/test_for_it.rs pinned 2, changes to ø); `for i in [1,2]: i*10` stays 20. Loops stay expressions
  (the user asked; "loops are statements" like Lean's Unit was offered and not chosen).
- Language name (user): the language is named warp everywhere; wasp remains only in README history and other history
  markdown. Defaults (supervisor): `*.wasp` → `*.warp`, with `.wasp` still readable; URLs and domains
  (github.com/pannous/wasp, wasp.pannous.com) unchanged; wisp unchanged. Card rename-warp, warp-fixer.
- P203 general rule, lax versus strict: annotations make it strict. Unannotated code is lax (Python-like, checked at
  run time); anything annotated (`x: T`, field types, `implements`, a declared parent type) is a promise the compiler
  enforces. `--strict` adds warnings for the lax spots. Asked after the user saw "a general conflict between pythonic
  type laxity and strict typing".
- P201 (from P203, card upcast-field): `s: Shape = Circle("a", 2); s.r` is a compile error "Shape has no field r"
  with the hint to match `Circle(r)` or use `s as Circle`; likewise `c: Color = rgb(1,2,3); c.r`. Unannotated
  `s = Circle("a", 2); s.r` works.
- P204 (refines P203) a value of static type any going into an annotated place (`y: any = 3; x: int = y`, `f(y)` for
  `f(n: int)`, `x = xs#1` of a mixed list, `level = event.level`) compiles with a run-time check: the value must have
  the annotated type when it runs, else an error; the annotated place never holds a wrong value. Card int-unchecked.
- P202 an `emit` that no handler receives gives a warning (chosen over P163's silence and a compile error).
- `when` by shape (user to warp-dc): `when click {…}` = `on click`; `when x > 3 {…}` = `whenever` (edge-triggered,
  P156); a block of `->` arms stays Kotlin's switch. Card signals-shape.
- Effect handlers steps 1+2 approved (user to warp-dc): block-scoped `on` handlers, and emitted events as named typed
  effects, lowered to plain calls. Card effect-handlers.
- Effect handlers step 3 (default, warp-e4, word choice): a block handler ends its `on … in` block with `break value`
  (abort via its own WASM tag); return resumes; raise/stop stay errors.
- Next big topic (user to warp-dc): stdlib-standard, the standard library as importable modules; warp-e4 leads.
- Reflection streamlined (user to warp-dc, card reflection, warp-70): `dir(x)` from compile time when known, else at
  run time from the WASM metadata. Words: x.type, x.class, x.fields/attributes/members, x.methods,
  f.params/signature, f.effects, event.listeners, module.exports, x.unit, x.doc. Lookup order field → meta →
  reflection. One warp custom section (warp.meta), kept unstripped.
  Built (default, warp-web): run-time reflection on the program's own classes is a compiled type dispatch
  (`o.fields` → `if o is P then ["x" "y"] else …`), no host call; warp.meta plus a host call serve values from outside
  the program (card reflection-foreign-meta). `members` is an alias of `fields` (field names only).
- P200b real references without exceptions (card real-references, warp-58): maps share like class instances
  (`m = {a: 1}; n = m; n.a = 2; m.a` → 2), and adding a new field keeps the same object (`q = p; q.color = "red";
  p.color` → "red"). `x.copy()` is the explicit independent copy.
- P200c (user): every object has `copy()` and `clone()` by inheritance from the root type (instances, maps, lists,
  texts), overridable by a class. Discussed first ("in C++ there are sometimes very complicated copy footguns"):
- P205 `copy()` is deep: everything mutable inside is copied, numbers and texts are shared (they never change);
  cycles and shared parts are copied once. Shallow is a default argument, not a method (user): `x.copy()` =
  `x.copy(shallow: false)`; `x.copy(shallow: true)` is shallow.
- P206 a resource inside (file, socket, timer, listener, closure) is shared by both copies, with a got-it note;
  --strict makes it an error.
- P207 one overridable `copy` method; `clone` is an alias that always calls it, so they cannot diverge; an override
  returns its own class (checked when annotated); a copy keeps the real class (no slicing).
- P208 `===` on objects (instances, maps, lists) is identity (chosen over a new `same(a, b)`): `q = p; q === p` yes,
  `p.copy() === p` no. On scalars `===` stays P196/P196b (type + value). `==` stays loose. The preferred spelling
  is `same`: `a same b`, `a same as b`, `a is the same as b`; `===` is its alias (user). `identical` was proposed
  and dropped as bloat (each phrase alias costs parser special cases). Plain `is` stays the type
  test (`red is Color`).
- check-assert (user to warp-dc): `check` acts like assert; float parameters stay IEEE and `==` stays exact (no
  tolerance); samples/polymorphism.warp uses `combine number with number`, so `check combine 1.1 with 2.2 == 3.3`
  passes with exact numbers.
- Quantities print without a space (user to warp-dc): `500m`, not `500 m`, in print, interpolation, serialize and
  the playground (warp-99).
- test-soft (user to warp-dc, card test-soft, warp-99): `test C` is a soft check that records pass/fail and
  continues; `check C` stays assert; `test "name" { … }` is a named test block beside functions.
- Copy details (defaults, warp-class, branch shared-maps): copy shares closures and Data nodes silently for now (P206's
  note would be a run-time warning, not built yet); `x.copy(true)` positional also means shallow, and in the
  Kotlin-style `p.copy(shallow = yes, y = 5)` shallow is the flag; each event handler still gets a copy of the event
  as raised; lists are still values until card shared-lists (P200b's "everything shares" includes lists).
- Soft tests (defaults, warp-worker, notes/soft_tests.md): bare `test C` lines are skipped by `warp run` too (P209);
  the playground's Run runs a program's tests and shows the summary (no separate Test button); page tests still run
  under a plain run for now; a program's own `test` function keeps the word.
- P211 `~` is looser than `≈` (card g_YHSM, "two levels of approximation"): `~` compares numbers within 1% (setting
  `rough_tolerance`) and texts also ignoring surrounding whitespace and punctuation ("Hello!" ~ "hello"); `≈` stays:
  numbers within 1e-9 relative, texts ignoring case and accents, lists and objects field by field.
- P212 a class may define `approximately` (≈) and/or `similar` (~); with only one defined, both operators use it;
  with neither, the built-in field-by-field rule applies.
- P209 test blocks run only under `warp test` (chosen over running with the main program); `warp run` and `use` skip
  them.
- P210 when all tests pass, one summary line ("✓ 12 tests passed"); failures print ✗ lines and "m of n failed" and
  exit nonzero.
- P199 bool slots accept 1 and 0 as yes/no everywhere (variables, fields, list items); other ints and texts are
  errors. Card bool-assign.
- P200 class instances passed to functions are shared references (like Python/JS). Card instance-field.
  Built as copy-in/copy-out (warp-c6): a parameter whose field a function changes is shared (maps too); aliases
  don't share (`q = p; q.x = 7` leaves p.x); only a variable argument gets the change back (`f(bags#1)` doesn't);
  a function passed as a value keeps value semantics. User: interim OK; real references (struct_backend's mutable
  per-class structs; WASM GC structs are references) on card real-references. Later the same day the user moved it
  to Next with the plan: $Node.value mutable, `p.x = 7` as an in-place struct.set instead of field_with, literal
  objects copied at construction; struct_backend not needed.
- `type(ø)` prints "empty" (word choice, not asked; warp-79, card type-static): matches the run-time kind name;
  unit, nil, ø may be aliases where a type name is read. `type(1.5)` stays "rational".

## Decided 2026-10-07 (user, multiple-choice interview, as recommended unless quoted)
- P169 `use math` loads a NEW warp math module (to be designed) with beautiful names, not the historic C names: "we
  need to educate people about beautiful names". The raw C library stays available as `use cmath`. Owner warp-67.
- P170 after `use list` its words work bare `zip(a, b)` and qualified `list.zip(a, b)`.
- P171 new words (zip, enumerate, unique, …) stay in their modules; a bare use without `use` is an error naming the
  module. Exceptions: `write` and `exists` are global (prelude). The file module also loads automatically when the
  program contains a file URL.
- Standard library released (to warp-03/warp-96): see the roadmap rule in notes/open_decisions.md.
- P172 alias words (`fire`, `trigger`, `signal` for `emit`) are not soft keywords: a program's own definition wins
  and the alias stops; the test `fire(x) := x * 2; fire(3)` → 6 stays.
- P173 an unannotated parameter called with several kinds silently becomes `any` (chosen over the recommended error
  "annotate it, e.g. want:any"). Card parameter-takes.
- P174 `mk(k) := { it * k }`: `it` stays the single parameter (mk(3) = 9), plus a warning when the body is a lone
  `{ … it … }` naming both readings and the lambda forms `return {it * k}` / `x => x * k`.
- P175 `xs where it > 1` is a filter with `it` only; a condition without `it` is an error naming the fix.
- P176 in a named tag block repeated `key: value` are children: `ul{ li: "First" li: "Second" }` has two li; a plain
  `{…}` stays a map with the duplicate-key error. Card g-_bGo.
- P177 `class Square implements Shape` / `struct Square: Shape` is a checked claim: compile error if Square lacks an
  operation of Shape.
- P178 enum cases with values are sealed classes: `Shape::Circle(r)` in a match is the type test plus binding.
- P179 sum types: the same mechanism as optional, auto-unwrapping (`a:int = Some(3)` gives 3). Payload fields by
  1-based position `rgb(1,2,3)#1`, with Rust-style `rgb.0`, `.1` as a fallback. A payload-free variant stays its
  symbol (`red`), and `red is Color` is true (membership in the variant list).
- P180 split-debuginfo stays "unpacked" plus the hourly sweep (chosen over the recommended "off"). Card shared-cargo.
- P181 the Integrator may merge branch write-hook-d2 (caf229b10, claudeignore-hook.sh: reason on stderr, printf).
  The Integrator wants the user's word directly (hook rule); warp-03 arranges that.
- P182 Set, Stack, Queue, Deque, Counter: module `collections` only; without `use collections` the error names it.
- P183 std modules are backed only by warp, host words or C compiled to wasm (never python3, node or system
  libraries); regex is the common subset of Rust regex and JS RegExp with a loud error for the rest; file append is
  `file.append(path, text)`, bare `append` stays the list method. argmax/argmin not switched: stay 0-based.
- P184 English number words are numbers (`one plus two` = 3) when no variable of that name exists, with a hint to
  replace them by digit literals.
- P185 `none(xs, f)` (the null spelling called with arguments) is an error naming the null (chosen over "no element
  matches").
- P186/P187 web defaults (DOM morph now with per-hole updates later, component identity by order, no separate
  context, wasm-split when on PATH; live update of the last line only for names, markup and choices): the user
  "is not wise enough to answer" / "idk": the defaults stand, not re-asked.
- P188 web rule (to warp-03): "Stay close to HTML and CSS; don't reinvent the web, only the basic machinery. HTML and
  CSS in our own syntax is fine." Triggered by the invented `li{ transition: fade 200ms }`. Applies to all web work.
- P189 extra control-flow words (try, catch, switch, match, yield, …) stay display-only keywords. The Kotlin way
  (keywords usable as names) is acceptable only where no conflict can arise; where an ambiguity is possible, enforce
  special quoting instead of guessing.
- P190 page-test words (test / render / click / fill … with / check): "idk": the default stands.
- P191 `square(x)` lives in the math module, not as a builtin. "Very important that we have many more functions in
  there, especially sin and so on": the math module may forward to cmath until warp has its own implementations.
- P192 netbase backend: later, as a first external-module experiment (Later card); if the server is revived, a
  smaller version, not the full multi-GB one.
- P193 the netbase warp package lives inside pannous/netbase.
- P194 (to warp-84): std/ merges into lib/ ("it would be libraries plural, so lib is probably better"; the user first
  said library/). netbase does not belong among the standard modules: "lib/extra/ or just a completely external
  module". Now lib/extra/netbase.warp; later an external module in pannous/netbase (P193, card netbase-first).
  Rule: long, readable names, as long as they don't get too long. Card lib-rename (worker session).
- Web words (defaults from P188 + the word-choice rule, no user answer; warp-19, branch web-words 23b405bb1): storage
  `local["k"]` canonical (localStorage), `storage["k"]` alias, `session["k"]` sessionStorage (natively in memory);
  clipboard `clipboard.write("text")` (navigator.clipboard.writeText), `clipboard.read()` = `clipboard` (native only;
  in a browser page reading stays a loud error, it is async).
  IndexedDB (word choice, warp-web, branch database-store 020211abc): `database["k"]` canonical, `indexedDB["k"]`
  alias, same forms as local/session (database.k, delete, keys); natively <program>.database.json beside the program.
- Trailing percent (default, warp-d0, card postfix-words): a `%` with nothing after it is a percent, x/100 exact:
  `10%` = 1/10, `200 * 10%` = 20, `50 % of 200` = 100 (`of` multiplies after a percent). CSS values like
  `width: 50%` stay as written; `whenever battery < 20%` keeps its battery-level reading. Infix `%` stays remainder.
- `time` (default, warp-ad, card time-day-value): the machine's local time of day as a duration since midnight (as in
  `on every day at 9am`); a program's own `time` wins; comparisons of constant quantities are decided at compile
  time; `time < 12` fails with the hint `time < 12h`, a non-duration gives a DimensionError; `print time` alone still
  fails until run-time quantities exist (units-p64).
- P56 revised (user's card unknown-entity, 2026-10-07: "must not stop the program"): an unknown entity `\:name` stays
  as written, in texts and outside them, with a compile-time warning; an error only with --strict. A bare `\alpha`
  stays the error "write \:alpha". Branch unknown-entity ecc9c2fcf (warp-ad).
- Bool type (user, card bool-type, via warp-84): "Yes is 1, no is 0", but true/false (yes/no) become a shallow type
  of their own: they act like 1/0 in arithmetic and comparisons, `type(yes)` is bool. They print as yes/no (warp-84,
  correcting the first relay's true/false); true/false stay accepted as input. == stays loose.
  Replaces the rule that True/False are encoded as Int 1/0. Default following from it (card zero-false, class):
  `0 == false` is true, `0 === false` is false (strict identity compares the type too).
- P195 `true + 1` = 2: a literal bool in arithmetic acts as 1/0 like a bool variable; the analyzer error "arithmetic
  on a boolean" goes.
- P196 `===` compares the full (static) type: `0 === false` is false. P196b: `1.0` is the exact int 1, so `1 === 1.0`
  stays yes; `1 === 1.5` and `1 === (1.0 as float)` are no.
- P197 warp-ee (class) updates AGENTS.md "True/False: encoded as Int 1/0" to the bool type once bool is on main.
- uniscript v1.0.4 released (user, via warp-84): restored LaTeX names (circ = ∘, varepsilon ε, varphi φ, plus to,
  neq, land, lor, lnot, ldots, dots, nat, complex, euler, degree, cbrt) and the algorithmic Unicode names; warp pins it.
- Emoji in code (user, via warp-84): a single emoji is a codepoint; multi-codepoint emoji (flags, skin tones, ZWJ)
  are texts, equal to their quoted form and not assignable names.
- P198 names HTML and LaTeX define differently: as recommended, HTML for letter names (ocirc ô, oslash ø), LaTeX for
  operators (asymp ≍, circ ∘). The user: context-dependent "is probably ugly but the best we can do"; list in
  notes/footguns.md how many such names there are.
  Follow-up default (warp-84): "letter names" means Latin letters with diacritics (ocirc ô, oslash ø, imath ı, jmath
  ȷ); cdot ⋅, varepsilon ε, varphi φ take LaTeX as exceptions. 36 names differ, listed in notes/footguns.md.
- uniscript 1.0.5 released (user, via warp-84) with the P198 letter/operator rule and its exceptions; warp pins it.
- try-finally (default, warp-a1, card try-finally): `try X finally Z` without catch is accepted; Z always runs and X's
  value is the result; if X fails (raised or trapped), the Error becomes the value, as in `try X catch e { e } finally
  Z`. Alternative: re-raise the Error after Z.
- Mutating a main-level list in a function (default, warp-7c, card uncalled-list-param): `names = ["a"]; grow() := {
  names.add(420) }` gets the same educating error as `names = names + [420]` there: "declare it `global names`";
  with `global names` it works. Alternative: Python's silent mutation without a declaration.
- Undeclared lists (default, warp-a1, card list-element-types): an undeclared list stays `list any` (`xs = [1];
  xs = ["a"]` is fine), while a scalar's first value fixes its kind.
- Method broadcast (default following P50, warp-c9, card method-broadcast): the method form of scalar library words
  (upper lower trim floor ceil round) broadcasts over lists like the prefix form: `names.upper` = `upper names`.
- Defaults shown to the user and kept (no objection): error highlighting (CLI carets under the word on stderr; web
  demo red/amber wavy underlines, message on hover; card g-_ZNg); P168 detail (an object whose fields are unknown at
  compile time keeps the field read `p.phone-number`); char as Text (card char-text, follows from P173: an untyped
  parameter given a one-character text takes it as Text, `g(t) := t as float; g("3")` → 3, `ord` still works).

## Decided 2026-10-06 (user, multiple-choice interview, as recommended unless quoted)
- P165b soft keywords vs tests: strict P165. tests/functions/test_named_arguments.rs:22 renames `fun` → `g`
  (`fun` is a hard keyword); tests/operators/test_root_word.rs:14-15 top-level `root = 5` / `root(x) := …` expect the
  error. Unblocks branch soft-keywords (warp-64, merge by warp-3f).
- P168 `p.phone-number` when p has no field phone-number but `number` exists: FALL BACK to subtraction
  `p.phone - number` (the user chose this over the recommended field error). Implemented by warp-64.
- P151 crates.io name (asked by the packaging session, answered there directly): "lang is perfect": the package is
  `warp-lang` (library still `warp`, binary `warp`), its runtime `warp-runtime`; notes/packaging.md.
- P71 `:=` without parameters is ALWAYS CHARGED (the user chose this over the recommended "now"): `y=3; z:=y*y; y=4;
  z` → 16, re-evaluated at every use, as wiki/charged.md §2 says; `z = 6` after it is an error. The per-use getter
  exists: commit 0b687e03 on branch late-binding (warp-29). Object entries `{s := clock()}` follow (§4 is revisited).
  Tests pinning "now" may be edited, each in its own commit naming P71.
- P63 (rest) a comma tuple `(frobnicate, 3)` in code is data, never called (worker default stands). Asked by warp-90.
- P94 WIT `char` maps to warp's Codepoint, not a one-character text. Asked by warp-d6 (branch web-components).
- P93 the "Batch board writes" paragraph is deleted from AGENTS.md.
- P95 tests/lists/test_map_starts.rs and its `mod` line: a worker checks it passes and commits it on a branch; the
  user's checkout is cleaned once main has it.
- P96 obsolete ignored tests (the Fixer warp-f6 listed 67): delete only group A, the 13 that are empty or assert
  nothing (test_merge_global/_memory/_runtime/_own/_wabt_by_hand, test_multi_value, test_get_element_by_id,
  test_canvas, test_dom_property, test_replace, test_extract_function_signature, test_parse, test_wast), plus
  numbers/test_math test_primitive_types. User chose "Only A" over the recommended A–C: the id/square-builtin tests,
  test_recent_random_bugs, test_array_constructor, test_all_wasm and the other unclear ones stay ignored.
  Correction (user, same day): test_wasm's test_dom_property did assert ($canvas.width == 300); it moves into
  web/test_web.rs, replacing that file's empty namesake, still ignored for the browser host (warp-f6, branch
  p96-obsolete-ignored).
- P102 (card g-1KS4) `warp build hello.warp` makes the native executable by default; on macOS/Linux it is named
  `hello` (no extension), on Windows hello.exe; `warp compile` makes the executable too (user chose this over the
  recommended "compile stays .wasm"); the .wasm only with `--wasm`; `--exe` still accepted. Asked by warp-f6 (branch
  build-exe-default).
- P103 (replaces P102's command words) user: "we don't need the build and compile flags at all. Just giving it a file
  will compile it." `warp hello.warp` compiles once, runs the program right away (output as before) and leaves the
  executable `hello` beside it (hello.exe on Windows); `build`/`compile` stay accepted as synonyms, not in the help.
  Assumed (undoable): `--wasm` and `--aot` stay for the module only. warp-f6, branch build-exe-default.
- P104 (after P103, warp-f6, branch p102-exe-naming) a plain `warp file.warp` always writes the small stub executable
  (1–5 MB); without a warp-runtime stub it prints a note and writes nothing, never a ~120 MB copy of warp. `build`/
  `compile` write the executable without running (exit 1 on failure). Worker assumptions standing: rebuild only when
  the source is newer; a program the runtime can't carry (fetch, read, run, warn) gets "note: no executable …" and
  runs; `--wasm`/`--aot` give the module.
- P105 user: "We also need `warp run` which shall do the opposite": `warp run hello.warp` runs the program and writes
  no executable (the opposite of `build`, which writes without running). Assumed (undoable): bare `warp run` without
  a file still opens the REPL. warp-f6, branch p102-exe-naming.
  Purpose (user): "the point is to avoid the ahead-of-time compilation because that's too slow": `warp run` must
  never take the machine-code path; warp-f6 measures it and tries a fast compile tier (Winch / opt_level None).
- P106 tasks share a variable with main only when it is declared `shared` (`shared done = false; go { …; done = true
  }; after done …`), scalars like P44's shared arrays; every other variable stays an isolate copy (P33). Asked by
  warp-d9 (branch async); as recommended. wiki/thread.md's example gets `shared`.
- P107 (issue #16, card g-1sPM) the playground offers only the .wasm download plus a one-line `warp hello.warp`
  instruction, no native executable (user chose this over a static Cranelift stub with the .wasm appended in the
  browser, and over a pannous.com build endpoint). Asked by warp-76.
- P108 (Sublime package, pannous/warp-sublime-text) Angle.sublime-syntax is retired; Warp.sublime-syntax (scope
  source.wasp) takes .wasp/.warp/.a/.angle (user chose this over the recommended split by extension). Asked by
  warp-76 for the Sublime worker. FYI there: ⚠️ no longer starts a comment (warp reads it as an error constant).
- Signals (warp-54, branch signals, notes/signals.md): P109 every variable can be watched (implicit), checks only
  where a listener watches; P110 `raise X` goes to `on X` handlers as a signal, without a handler it stays the
  catchable exception; P111 (revises P38) writes through `global x` in called functions run the listeners too;
  P112 no batching block (user chose this over `together { }`): only a multi-assignment `a, b = 1, 2` notifies once.
- P113 the CLI and console show nothing for a ø result (like Python's None): `warp 'print 3'` prints only "3", and
  `x = ø; x` prints nothing. Asked by warp-d6 (after #18, print gives ø; show() in src/main.rs); as recommended.
- P114 (card g-1tHQ, warp-d6) user: "we already have a general Meta mechanism so the meta keyword or attribute should
  persist and be filled with the comments when we activate them but usually they should be deactivated": `x.meta` /
  `x.@key` stay; comments fill them only when activated (off by default, no compile cost); plain keys inside `.meta`
  (`.meta.comment`), `@` for direct reads. Assumed (undoable): activation by the pragma `use comments`; bindings only
  for now (fields/functions unanswered).
- Classes (issue #14, card g-1nug, warp-40, notes/classes.md): P115 bare field names read the receiver, `self`
  (alias `this`) too; P116 `c.inc()` updates the variable c (objects are values, like `xs.add(v)`); P117 a body
  definition without parentheses (`area := side*side`) is a getter computed at each read, and `class b extends a`
  copies a's fields and methods (b's own override) (user chose inheritance over the recommended none).
- P124 (warp-dd, branch functions 9f208fbc3) a lambda that assigns a variable of its enclosing function shares it
  (`def counter(){ n=0; ()=>{ n+=1; n } }` counts 1, 2, 3 like JS/Kotlin/Swift), `nonlocal n` may also be written;
  this relaxes wiki/charged.md's rule for lambdas. Worker assumption standing: a nested `def` changing one without
  `nonlocal` is a loud error naming `nonlocal n` (Python's rule).
- P125 (warp-dd) optional parameters, user: "x=ø, maybe x, or x? Same as with optional types." Three equivalent
  spellings of a parameter that may be missing (missing = ø): `x=ø`, `maybe x`, `x?`; typed like the optional types
  (`x:int?`, `maybe int`). Such a parameter holds ø or a T, so `f()` and `f(2)` both work. Assumed (undoable, not
  addressed by the user): `a ?? b` is a unless a is ø (only ø, not 0/false); Kotlin's `?:` is not added.
- P126 (warp-32, follows P123) texts inside containers are quoted everywhere (print, interpolation, string()):
  `P{x:1 name:"a"}`, `["a" "b"]`; a top-level `print "a"` still writes a.
  Follow-ups (warp-ab), settled by P126's read-back rule without a new question: a character in a container prints
  with double quotes too (`["c"]`, since one-letter "a" parses as a character), and a `"` inside a quoted text is
  escaped as `\"`.
- P127 (warp-32, classes) class int fields are fast i64 struct fields; a write that does not fit is a loud run-time
  error "x of Point is an int field: 2^70 does not fit in 64 bits; declare it x:bigint or x:number".
  Moot as implemented (warp-32): int fields are i64 slots carrying warp's exact-int encoding, so 2^70 is kept
  exactly; no overflow, no error. The speed goal is met without a fixed width, so no follow-up question.
- P129 (signals, branch signals-broadcast) user: both forms, plus `send signal chat`, or just `send "file system full"` when there
  is no value. Both `broadcast value on "chat"` and `send value to "chat"` (`to` read as the target
  after `send`, not a range) reach `on message from "chat" {…}` in every program on this machine; without a channel
  name both use "warp". Interpreted (undoable): `send signal chat` / `send "file system full"` without a value sends a
  named signal with no payload, received by `on "file system full" {…}`. P129b (user: "Yes, build it now"): named
  events cross programs: `broadcast stop the machine{…}` reaches `on stop the machine from "chat" {…}`.
- P130 (card footgun-pi) user: "loud error, if it was declared constant before which it should be": pi and the other
  named constants are declared constants, so `pi = 4` is a compile error "pi is a constant; fix: another name".
  Assumed (undoable): a class field named pi (`class circle{pi = 3}`) is the class's own field, not an assignment to
  the constant, and works without a message.
- P131 (card int-declaration) an int variable or field refuses a fraction loudly: `x:int = 3; x += 0.5` and
  `class P{x:int}; P(0.5)` → "x of P is an int field, got 0.5 (a fraction)", a compile-time error for literals, a
  run-time error for computed values.
- P132 (card function-values) `{f: x => x * 2}` is the field f holding a lambda; a type word or capitalized word
  after the colon (`{p: Person => …}`) still makes a typed lambda.
- P133 `|x| x*2` at the start of an expression is a lambda (Rust/Ruby; `|a, b| a*b`).
- P134 `on exit {…}` runs once when the program ends (main returns, `exit(code)`, ctrl-c), `event` = the exit code.
- P135 `on every day at 9:00 {…}`; `at 9:00 {…}` runs once.
- P136 "ittt" rules over system values (`whenever battery < 20% {…}`, `whenever online {…}`) are split into one card
  per value (battery, network, dark mode, clipboard), built later.
- P137 tests/numbers/test_angle.rs test_function_params expects the Int 9 for `f(x)=x*x;f(3)` (the text "9" was a
  C++-port artifact) and is un-ignored.
- P138 (card functions-go, warp-a8) Go's `total := 0; total += n` stays a loud error ("total is charged (total := 0):
  it runs at every use and cannot be assigned; write total = … at the definition"); P71 stays strict.
- WebAssembly modules (warp-25, branch wasm-modules, notes/wasm_modules.md), all as recommended: P139 `import m` /
  `use` / `require` only declare (ø); `include m` runs m's main/start export and is its value (test_import_wasm → 42).
  P140 assigning an imported global: a mutable wasm global is set in the module, an immutable one is a loud error
  (P130's rule). P141 REVISED (user: "if the built-in wins, then what's the point of even allowing to define it? It
  should be an error then, no?" … "at the definition side if we don't define it, then you decide"): a warp
  definition named like a builtin (`def double(x)…`) is a loud error at the definition. A foreign .wasm export
  can't be renamed, so (Interviewer's choice, as the user delegated) `import m` works, `m.double(21)` always works
  (qualified calls get built), and a bare `double(21)` is a loud error "double is ambiguous: m.double(21) for the
  export, 21 as float for the cast". No warning-and-builtin-wins.
  P144 (warp-25) the old test_import_wasm (tests/wasm/test_wasm.rs) is edited to the P139 values (import/require →
  ø, include → 42, fixture from tests/fixtures/wasm) and un-ignored, in its own commit naming P139.
- P145 (warp-41, branch functions) R's `x <- 3` assigns with a got-it note "write x = 3"; the cramped `x<-3` is a
  loud error naming `x = 3` and `x < -3`; `x < -3` compares. User afterwards: "we can keep it as an assignment for
  now, but maybe we reserve it for special things later": `<-` is reserved; the assignment reading may give way to a
  special meaning later, so docs and examples never use `<-`. FYI (worker, no question): Haskell `twice x = x * 2;
  twice 4` defines a function when a later statement uses the name and the body uses every parameter (e6f1cbbbe).
- P146 (card hijack-stdlib, warp-06) next under "hijacking existing standard libraries": C libraries compiled to
  WebAssembly, imported as core modules with the C header's types (char* texts, pointers), so the same `use zlib`
  runs natively, sandboxed and in the browser (plan: notes/stdlib_connectors.md "Status 2026-10-06").
  Dropped as answered by P124: a block/lambda argument writing a captured variable (`total = 0; each_twice { |x|
  total += x }`) shares it, no `global` needed (warp-93, branch ruby-yield).
- P147 (P146 step 4, warp-06, branch hijack-libc) the prebuilt wasi-libc module lives in this repo:
  web/playground/lib/libc.wasm + libc.h, built by a script and committed like warp.wasm; the browser host loads it
  for `use c` instead of host.js's hand-written shims; native keeps dlopen.
- P148 (warp-93, branch change-old) in `on change x {…}`, `old` (alias `previous`) is x before the change (Vue's
  watch); a program variable named old/previous keeps its own meaning. Assumed (undoable): `on set x` gets `old` too.
  By the word-choice rule (Standing rules): `previous`, `was` and `before` are aliases of `old` that work with a note
  naming `old`.
- P142 (scope of P141, asked by warp-e4) the definition error applies only where the builtin would win, i.e. the
  definition could never take effect (casts like `double`, operators, type names). Library words (`add`, `map`,
  `count`) may be redefined and the user's version wins; class methods are always allowed (called as `x.add`). User: `add`
  was only an example, not a builtin; map should not be the main tool, use `square all x` or broadcasting. So docs, hints, examples and new tests prefer broadcasting
  (`square [1 2 3]`, wiki/broadcasting.md) or `square all x` (wiki/all.md) over `map`.
- P143 (warp-a8) a function called without parentheses next to a comparison, `f x == y`, is a loud error naming
  both forms: "square 3 == 9 is ambiguous; write (square 3) == 9 or square(3 == 9)" (user chose this over the
  recommended "(f x) == y for all functions"; warp-a8 had implemented (f x) == y for user functions only). Assumed
  (undoable): statement words (print, return, assert) still take the whole expression (`print 3 == 3` prints true);
  the wiki examples `square [1 2 3] == [1 4 9]` (wiki/all.md, wiki/broadcasting.md) get the parentheses.
  Refined (user, same day): "But maybe it's not ambiguous when square takes a number and not a bool." Types decide
  first: when the parameter type rules out the comparison's bool (`square number = …`, `f(x:int)`), `f x == y` is
  `(f x) == y` without a message; the error stays only when both readings type-check (parameter untyped/any or
  bool). So `square 3 == 9` is true and the wiki examples keep their bare form.
  P149 (warp-41) an untyped parameter's type is inferred from the body before P143 applies: `x + 1`, `it*10`,
  `it*it` need a number, so `def f(x){x+1}; f 1 < 5` and `square := it*it; square 3 == 9` are `(f x) op y` without
  a message, for every comparison; the ambiguity error stays only when the body accepts anything (`print x`). The
  footgun test `f := it*10; f 3-1 > 15` → true stands.
- Async (warp-93, porting Promise/asyncio/Go/Kotlin cases; first sent to warp-93 as P150-P152, renumbered because
  the packaging session's P150 license / P151 crates.io name came first):
  P153 race: `await any [go a(), go b()]` is the first task to finish, the others keep running. The user's answer on
  `await first [tasks]` (asked again, user picked the recommended option): `first` keeps its list meaning, and on a
  list of tasks a compile-time got-it note says "for the first to finish: await any […]".
  P154 deadline: `await job within 100 ms or 0`; on timeout the job is stopped and the `or` value is the result,
  without `or` the timeout is an error. User: "would the syntax 'or stop with 0' be overkill? maybe we can just give
  it as a hint that this is the behavior": no extra `stop` word. User, clarified: the hint is a compile-time got-it
  note on the `within … or` expression ("on timeout the job is stopped and the result is 0"), not a run-time message
  when the timeout happens; a timeout itself stays silent.
  P155 channels are one concept: `ch = channel()` is local, `channel "chat"` machine-wide, both with send (blocks
  until received), receive, `for v in ch {…}` and close; `send v to "chat"` (P129) is that channel's send.
- P167 (classes, warp-e0) Go's positional literal `Point{1, 2}` builds `Point(1, 2)` when Point is a known class,
  with a got-it note "warp writes Point(1, 2)"; for an unknown name it stays tagged data.
- P165 Kotlin-style soft keywords (user's proposal: with so many synonyms in so many situations, allow overwriting
  keywords except the main ones). Hard keywords, never redefinable: control flow (if, then, else, while, for, in,
  return, break, continue), declarations (def/fun/fn, class, var/let/const/val, global), literal values (true,
  false, ø/null/nil) and modules (use, import, include). Every other word (emit, send, init, new, root, listeners,
  every, …) is soft. User: soft keywords "should not be completely redefinable, just usable in a narrow context".
  Interpreted (undoable): a soft keyword may name a local variable, parameter, field or method (`send = 3` inside a
  function, `class Mail{ send(){…} }`, `{emit: 1}`), and there the user's name wins; at top level it can't be
  redefined globally (loud error naming another name). Each use as a name gives a got-it note ("send is a keyword;
  here it is your variable").
- P166 (functions, warp-64) element-wise operators `.+ .- .* ./ .^` on a plain number act as the plain operator
  (`6 ./ 2` → 3, `sq = @(x) x.^2; sq(3)` → 9), like MATLAB and NumPy; on lists they still map.
- P163 (signals, warp-ed; from the user's remark that raise and throw mean errors) `emit alarm{level: 3}` runs the
  `on alarm` handlers and continues; an emit nobody handles does nothing. `raise`/`throw` are always errors, so an
  `on X` handler no longer turns `raise X` into an event (replaces P110's dual meaning). fire/trigger/signal are
  aliases of emit with a got-it note; send/broadcast keep their machine-wide meaning (P129). User, adding: "emit and also
  send": `send alarm{level: 3}` without `to` is a first-class synonym of emit (no note), matching P129's value-less
  `send "file system full"`; only `send v to "chat"` / `broadcast v on "chat"` go to a channel.
- P164 (functions, warp-64, wiki/argument.md) in a parameter shape `phone number` is the field phone of type number
  (a type word after a name is its type everywhere); the wiki example calling it with a text changes to `phone text`.
  User, adding: `phone-number:text` works too, a hyphenated field name with a `:` type. Assumed (undoable): in a
  declaration context (parameter shape, class field, key before `:`) `a-b` is one name; in an expression it stays
  subtraction, and reading such a field uses `p.phone-number` / `p["phone-number"]`.
- P162 (classes, warp-e0) `init(…){…}` in a class is always the constructor (user chose this over the recommended
  "unless called explicitly"). User, correcting: `init` is THE constructor name, not `value`. `value`, JS
  `constructor` and Python `__init__` still work as aliases with a got-it note "warp says init" and an "I meant:
  init" fix; docs, hints and examples use `init`. User, extending: all common constructor names are aliases
  of `init` with that note: `value` (wiki 2023), `constructor` (JS/TS), `__init__` (Python), `initialize` (Ruby),
  `__construct` (PHP), `New` (VB.NET), `Create` (Delphi), Rust's `new` inside a class/impl, and a method named like
  its class (C++/Java/C# `Point(x, y){…}` inside `class Point`). At the call site `new Point(1, 2)`
  builds the same value as `Point(1, 2)`, with a got-it note that `new` is superfluous and a fix removing it; `init`
  stays the definition name (it initializes an existing instance; Rust's `new` is a factory).
- P161 (functions, warp-41; the user told the worker directly) no Swift-style argument labels: "we don't do this
  here, I don't like that redundancy". Ported labels (`func greet(person name: String)`, `_ x: Int`) compile like P157,
  with a got-it note "warp names a parameter once"; docs and examples never use labels.
- P160 (warp-42, wiki/reference.md; survey notes/implicit_params.md) `$0`/`$1` were overloaded three ways (lambda
  parameters, WebAssembly positional arguments, node ids). The user chose a different syntax for ids: `@1` is the
  node whose `@id` is 1 (`a[id=1]{ b c { parent=@1 } }`), `ref 1` / `ref a` say the same in words, and `$a` stays
  the reference by name (nearest enclosing node with key a). `$<digits>` is only ever positional, never an id.
- P157 (functions, card functions-generic, warp-41) no generic syntax in warp: untyped functions (`def id(x)`,
  `max(a, b)`) already work for every type, with copies made per call. Ported `fn id<T>(x: T) -> T { x }` compiles as
  the untyped form with a got-it note "warp infers types: write def id(x)"; warp's docs never use `<T>`.
- P158 (warp-42) `then` pipes only when the right side is a function stage missing its argument (`… then sort`,
  `… then filter(x => x > 5)`) and no `else` follows; otherwise it is the condition (`it<2 then 1 else …`). In the
  collision `x > 2 then print` the condition wins, with a got-it note naming `|>`.
- P159 (card wiki-mutable, warp-42) `const x=7; x=7` works: assigning a constant its identical value gives a warning
  with the fix "remove the redundant assignment" (a different value stays P130's error). Changing a `let` variable
  (`let x="hello"; x+=" world"`) works, with a got-it note teaching `var` for variables that change.
  No question needed (word-choice rule): `root` is an alias of sqrt (`2|square|root`, wiki/pipe.md); a user
  variable or function named root wins (P142).
- P156 (signals, warp-ed) `whenever cond {…}` is edge-triggered: it runs each time the condition becomes true.
  `x = 0; whenever x > 5 { print "big" }; x = 6; x = 7; x = 3; x = 8` prints big twice (at 6 and 8);
  `a=0; b=0; whenever a+b==1 {…}; a=1; b=0` runs once.
  The wiki (where `whenever` is documented, and Footguns.md) shows what to write for while-like behavior instead
  (every change while it holds: `on change x { if x > 5 {…} }`; as long as it holds), verified examples.
- P70c (unparked; asked with a realistic example after the user found the old one "completely constructed")
  handlers and definitions created in a loop keep their own iteration's loop variable: `for i in 1 to 3 { button
  "Item {i}" on click { print "clicked {i}" } }` prints "clicked 1" for Item 1 (JS `let`, Swift, C#; not the JS
  `var` / Python late-binding bug). Variables the loop does not own follow P124 sharing. The old test (→ 3) stands.
- P64 (unparked) units, user: "probably evaluate at compile time but keep as meta data under certain circumstances;
  units are very valuable in physics, we should not drop them too quickly". Units are checked at compile time
  (static, F#-style: `3 m + 2 s` is a compile error, no run-time cost) and are kept as metadata on the value where it
  leaves static knowledge (printing `5 m`, serialization, a value of unknown/any type, data read back), never
  silently dropped. Card in Later: revive branches static-units-1..5 (green 2026-10-05, 1321 commits behind main)
  plus the metadata part.
- P128 (warp-3a, card g-3HmY) listeners: `listeners of x` is the list of functions listening to x (`count listeners
  of x`, `for f in listeners of x`), `clear listeners of x`; one listener is removed by its name:
  `alarm = whenever t > 30 {…}` then `remove alarm from listeners of t`.
- P122 (classes, warp-8e, card g-1nug) static members take the explicit keyword (user chose this over the
  recommended "`pi = 3` in the class body is a constant"): `class circle{r:int; static pi = 3}; circle.pi` → 3, also
  `c.pi`, never stored per instance; a plain `pi = 3` stays a per-instance field with a default value. `static` no
  longer gets the "no meaning in warp" note (P78).
- P123 (classes, warp-8e) an object's text is the constructor form `point{x:1 y:2}` everywhere: as a result, from
  `string(p)` and from `print p`; it reads back in as the same value.
  Follow-up (user): "Maybe Point{x:1 y:2} to distinguish it from untyped data (but that's just optional convention
  when printing.)" Assumed (undoable): the printer writes the class name as declared, so it still reads back; the
  convention is to declare classes capitalized (`class Point`) in docs, examples and new tests, so a typed object
  `Point{x:1 y:2}` stands apart from untyped tagged data `point{x:1 y:2}`. The printer does not capitalize on its own.
- P118 GPU maps (warp-d9, notes/simd.md): only after SIMD lands, only on an explicit `@gpu` map, never silent
  offloading (f32 differs); native first via wgpu. Card in Later.
- P119 NaN canonicalization only where a float's bits are observable (print/text, bit reads, memory stores, host/FFI
  calls, task crossing); arithmetic in between runs free (engine flag off, emitter canonicalizes). warp-d9; float
  loops measured 7.2 → 3.8 ns per item.
- P120 a program whose main ends while handlers listen: the CLI (`warp file`, `warp run`, executables) keeps running
  and prints once to stderr "listening: … (ctrl-c to stop)"; in-process eval and tests never wait. warp-54.
- P121 `exit` / `exit(code)` end the run, not the process: the CLI exits with the code, an in-process eval returns.
  warp-54.
- P97 (after P12) gc_struct!/wasm_struct!/wasm_object! stay as thin sugar on GcObject; only the unused gc_traits
  behind them go; no test edits. Asked by warp-40 (code-quality).
- P98 commented-out code blocks of 3+ lines and comments restating the next line are deleted from src/, one commit.
- P99 renames incl. mechanical test-file edits: laste→last_item, Dada→DataValue, peq!→parses_to!,
  s!/strings!/Strings!→texts! (s! stays for to_string), wis!→wisp!; unused todow/tee deleted.
- P100 the test macros is!/eq!/skip!/check!/put! move from src/extensions.rs to tests/common/mod.rs (`use crate::is`
  in tests, mechanical edit; web/playground's runner checked).
- P101 the user's rough comments in Cargo.toml and src/extensions.rs are reworded neutrally; the dead wasm-ast line
  goes.

## Decided 2026-10-05 (user, multiple-choice interview, all as recommended unless quoted)
- Data vs code (P51, P62, P63 (1-2), charge levels, D7 revised): specified in wiki/charged.md, the single source;
  RELEASED for implementation by the user on 2026-10-05 ("you have the green light, go ahead and implement
  everything"). Tests the spec changes may be edited, each edit in its own commit naming the release. User words on
  the way: P51 "should obviously be an error unless we're in a clear data context"; "def == deferred"; "def should
  behave exactly like Python … but close"; "y=3; def z(): y*y; y=4; z() gives a compiler error unless we explicitly
  define it as global"; "with some precomputed, precompiled paths"; naming `block`/`code`, `data` for what never
  runs, `quote` dropped. Work packages assigned by the Supervisor (blocks warp-90, late binding + precomputed paths
  warp-29, run-time `!` later).
- P70 (late binding, warp-29): (a) a change is an error only where a later call can see it; (b) `global` may stand on
  the variable at main level or in the reading function; (d) a pure getter over constants may be computed once
  (folding). All as recommended; spec wiki/charged.md 77061c2. (c) later.
- P72 memoization: no syntax; the compiler decides alone which pure functions to memoize (asked by warp-29; the user
  chose "Compiler decides alone" over the recommended `memo def`). Constant calls are folded in `warp compile` output
  only, not in eval/is! (worker assumption).
- P73 `x!` has one meaning, force: a block runs, an optional unwraps (or errors), a plain value is itself; the kind
  decides, statically where known, else at run time (asked by warp-90). Spec wiki/charged.md 37ccaf7.
- P74 user: "use the opportunity to make codepoint the default name. since this is a sub type of integer, it should
  be castable to float", clarified: `codepoint(c)` is the preferred name in hints, fixes and docs (`ord`, `ordinal`
  stay synonyms); it gives an Int, so `codepoint('x') as float` is 120.0; `'x' as float` stays an error whose hint and
  fix offer `codepoint('x') as float`; P27 stands. tests/text/test_text_casts.rs may be edited for the hint text (user
  decision). Asked by warp-ea (fixits).
- P75 fix buttons read "I meant: <replacement>", the meaning as a tooltip (warp-ea's default).
- P77 `warp build --exe prog.warp`: the executable prints what the program prints, then its value as `print` shows
  it (texts without quotes); exit code 0, 1 on a trap (asked by the aot worker warp-ec; as recommended).
- P78 modifiers from other languages before a definition (`public`, `static`, `extern C`, `inline`, `virtual`,
  `final`, `private`, `volatile`, `native`, …): accepted and skipped with a got-it note "public has no meaning in
  warp"; words with a warp meaning (`global`, `const`) keep it; a lone definition stays ø, so test_modifiers is edited
  to call the function (user decision). Asked by warp-90; as recommended.
- P79 the 13 inherent browser-suite failures (notes/web_playground.md: test_download, test_use_modules, test_package_pin,
  test_law lean, test_eval_state threads, test_use_scopes) are marked `#[cfg_attr(not(feature = "native"), ignore =
  "browser: <reason>")]`: they still run natively, the browser suite turns green (user decision, existing-test edit).
  Asked by warp-ec; as recommended.
  Refined (user: "but download should work on the browser and temporary folder may be through browser files"):
  test_download and test_use_modules are made to pass in the browser instead (download via fetch; temp dirs on a
  browser file system such as OPFS or the in-memory WASI fs); the directory walk of test_use_scopes uses the same
  file system if it fits. Only git, Lean and the threads case are ignored in the browser.
- P80 exact ratios and big Ints cross to tasks: control::test_threads::an_exact_number_beyond_the_fixnums_cannot_cross_yet
  becomes is!(…, 3.5), renamed an_exact_number_crosses_to_a_task (user decision, existing-test edit). Asked by the
  Fixer warp-2d (card g-rH6E); as recommended.
- P81 the kebab-key warning (`a=5; b=1; a-b:2; a-b`) offers three fixes: "the data key": `"a-b":2` (only when nothing
  reads `a-b` bare in that block); "the subtraction" at each bare read: `a - b`; and "the data key, renamed a_b": the
  key → `a_b:2` and every bare read → `a_b` (always applicable, renames the key in the data). Asked by the Fixer
  warp-2d (card g-qU1o, was parked in Later; the user answered anyway): option B, "also offer renaming".
- P82 returning a function: a function reference is written `function add` or `&add` (wiki/function-pointer.md); a
  bare name that needs arguments is the error "add needs 1 argument" everywhere, in a body's last value too, nested
  or top-level alike, with the fix "I meant: function add". Asked by warp-90 (card returning-top). User: "function
  references have an extra keyword … Maybe it was just function", then the recommended option.
- P83 (after P82) an alias needs the explicit reference, `g = function add` (bare `g = add` is the P82 error with the
  fix); an argument to a parameter that takes a function stays bare, `apply(add, 3)` (the parameter says a function is
  expected, like wiki/function-pointer.md's `map square on xs`). Asked by warp-90; as recommended.
- P84 `sum := fold +; sum 1 2 3` → 6 (wiki folding.md). User: "This should have already been done with broadcasting":
  several juxtaposed arguments to a one-parameter function are taken as one list, `f 1 2 3` = `f [1 2 3]`, through
  the broadcasting machinery (not the recommended keep-the-error). Asked by warp-90.
- P85 ignored-test corrections: only (1) approved: test_math_primitives expects -42.1 (typo) and is un-ignored.
  (2) test_function_params "9" vs 9, (3) test_string_operations "say 0." vs "say 0", (4) the fetch tests' trailing
  "\n" were NOT approved: those tests stay ignored as they are. Asked by the Fixer warp-2d.
- P86 Node::add of two values it doesn't know (Symbol + Number), user: "it depends on the context: in lazy context it
  must be allowed, in evaluation context the types must be of the addable trait". In a lazy context (data, a block,
  symbolic) it builds the unevaluated sum `a + b`; when evaluated, both operands must have the Addable trait, else a
  loud type error, never a panic. Asked by the Fixer warp-2d (card node-add).
- P87 test_wisp_defn checks `body.drop_meta().serialize() == "mul(it it)"` and is un-ignored (user decision).
- P88 capabilities, user: "currently allow everything to everyone": for now every host (CLI, eval, the playground,
  eval_untrusted) grants every capability, Process and Sql included; the capability checks stay in the code for
  later. Asked by warp-90 (card hijack-stdlib).
  Follow-up (user, multiple choice): eval_untrusted too ("Everything, untrusted too"). The five tests pinning the old
  refusals change in commits naming P88 (run-time block, libm/strlen, test_data_does_not_execute's eval part,
  exec/execute now "undefined: exec …", untrusted Python/JS); parse_data still never executes. The old asserts come
  back once a host narrows the grant.
- P89 foreign runtimes (`use python "math"`) get their own capability, not Ffi (granted to everyone for now, P88).
  Asked by warp-90.
- P90 the debug module ./test.wasm stays written at every compile (user chose "keep always on" over the recommended
  WARP_DEBUG_WASM switch). Asked by warp-ec.
- P91 one shared analysis per compile (extract_user_functions, EffectReport::of), refreshed only when a pass changed
  the program: yes, as its own card after warp-90's and warp-2d's lowering work lands, one pass converted first to
  show the gain. Asked by warp-ec; as recommended.
- P92 `foo()` with explicit empty parentheses is a call: an undefined name is "undefined function: foo" (also a
  mistyped or unlinked zero-argument C call); a bare `foo` stays a symbol. Asked by the Fixer warp-2d (card
  unknown-zero-arg-call); as recommended.
- Got-it scope: the prompt offers `[y = this one, a = all of this kind, n]`; one expression is remembered by its
  written text; a `// got it` comment silences that line in the source.
- P65 arithmetic a text can't do on a character (negation, %, /, sqrt) is not_a_number; `ord(c)` gives the number;
  comparisons keep code points.
- P66 Int division by zero is always the catchable divide_by_zero; only float division (1.0/0.0) is ∞.
- P67 `catch e` / `except E as e`: e is the Error value (its message).
- P68 a braceless call whose argument contains an operator (`square 3 + square(4)`) gets a got-it warning naming
  `square(3) + square(4)` and `square(3 + square(4))`; the reading stays.
- P69b a block arriving at run time is pure by default, more rights only by explicit grant; run-time data is tainted.
- P69c user: "warning and lazy loading of a shared compiler": `warp compile` with run-time `!` warns, the module loads
  a shared compiler lazily at the first `!`.
- P48 custom operators: the wiki syntax as proposed (`prefix operator ⁻ := it*-1`, `suffix operator ³ := …`,
  `infix operator ⊕ := left+right`), prefix/suffix tightest, new infix like `+`, the symbol known file-wide (pre-scan).
- P29 `pair.0` counts from 0 (like `pair[0]`).
- P30/P35 keep as implemented: `xs.pop()` removes and gives the last item, `m.remove(k)` removes and gives the value
  (ø when absent), `xs.index_of(x)` is 1-based, 0 when absent.
- P44 shared arrays keep `shared xs = int[n]` with atomic `xs#i += v`, Ints only for now. Re-asked: `atomic` is a synonym of `shared` (below).
- P56(2-4) `\:epsilon`/`\:phi` are ε U+03B5 / φ U+03C6; an unknown entity name is a loud error; `∞` is f64
  infinity now (ω stays the hyperreal infinite).
- P57 hyperreals: `1/(1+ε)` stays an error; dual-number mode and run-time hyperreals later (with exact reals at run
  time).
- Standing rule (user): "it's allowed to un ignore test that are suddenly passing". Removing `#[ignore]` from a test
  that passes unedited needs no question; editing its assertions still does.
- P49b a whole float is no int either: `f(x:int)` called with 2.0 is refused like `x:int = 2.0` (write `2.0 as int`).
- P45b the P45 error covers kinds evident from the source; inferred kinds keep the Node fallback until inference is
  reliable.
- P59 the Fixer's five ignored tests get the recommended edits (user decision): test_array_creation expects "index
  out of range" (keeps `pixel:int[100]`, drops `pixel array;…`); test_array_initialization_basics uses
  `count(x)`; test_array_initialization: typo fixed, the two natural-language array phrases dropped;
  test_array_type_generics expects "list of int"; test_hyphen_units stays ignored (interval equality, later).
- P52 a phrase-defined function is called with its own prepositions (`add 1 to 2`, `square of 4`); everywhere else
  `to` stays a range and `of` a field lookup.
- P53 test_custom_operators: the `.5³` line becomes 0.125, the test is un-ignored (user decision).
- P54 test_precedence_declarations_are_refused asserts the working `operator ⊕ has precedence above +` (user
  decision); only declared operators are ranked, built-ins never re-ranked.
- P55 test_while_nop_issue is rewritten to `x=0;while x++<11: nop;x` → 11 (user decision); a loop's value stays its
  last body value.
- P50 a declared scalar parameter broadcasts over a list too (`foo(x:int):=x+1; foo([1 2 3])` → [2 3 4], the wiki's
  `square number = …; square [1 2 3]`); test_argument_kinds and test_parameter_call_kinds change to a text argument
  (user decision). `print [1 2 3]` keeps printing the list (not asked; the recommended default stands).
- P49 a float passed to an int parameter is a compile error ("2.2 is no int: write 2.2 as int"); the ignored
  test_function_argument_cast is edited accordingly (user decision).
- P22 `p:photo = pic{width:3}` with pic a known other type: an error that teaches `pic like photo`. The warning
  default on claude/like-keyword is undone.
- P23 the type-word clash error covers every type word, generic ones (`number := …`) included (as assumed).
- P24 the suffix form `4 doubled` calls a user function too (like `4.square`, wiki D9 `1+2 squared`). Done (branch
  suffix-calls): every English past form, +d/+ed, a doubled final consonant (`stopped`), y → ied (`copied`);
  tests/functions/test_suffix_word_spellings.rs. The example itself needs a function named `double`, which P20 forbids
  (a type word): `double(x):=…` is "double is a type; rename your function".
- P26 libm (sin, exp, …) counts as pure: no Ffi capability, allowed in eval_untrusted and `! pure` functions.
  The Ffi assumption (effects.rs) is undone.

## Decided 2026-10-05 (user, own words)
- P45 a variable given values of two kinds that do not mix (`x=[1]; x=5`): user: "compile error unless we are in
  script mode, which is not defined yet". The implemented Node fallback is undone outside a future script mode;
  the error names the variable and suggests another name. Int→Float widening stays. Script mode: a new open topic.
  Done (branch p45-kind-change): analyzer::check_kind_changes on the program as written, before lowering, compares
  the kinds evident from literals (`x = [1]; x = 5` → "x was a List, is given an Int: use another name"). Where a
  kind is only inferred (a call's result, a loop variable reusing a list's name) the Node fallback still holds the
  value: an error there would rest on guessed kinds (function results default to Int before inference).

- P58 `xs#a..b` (range from xs#a, or a slice?): user: "create a strong warning and I don't care how to interpret
  it". So: unparenthesized `xs#a..b` keeps the range and gets a strong warning naming both explicit forms,
  `xs#(a..b)` / `xs#(a…b)` (the 1-based slice) and `(xs#a)..b` (the range).

- P56(1) uniscript entities: user: "yes, they should work everywhere but they have the syntax \:". The entity form
  is `\:name` (wiki/uniscript.md: `\:infinity == ∞`, long form `<:name>`), in code AND inside texts; a bare
  `\name` is no entity (so `"\nat"` stays newline+"at"). The branch fix-uniscript-entities moves from `\name` to
  `\:name`.
- P28 `real x;` read before any assignment: user chose "Zero value (Go)": x reads as the zero/empty value of its
  type. The loud error (analyzer::check_unassigned_declarations) is undone. Done: declarations::lower_bare_declarations
  makes `T x` of a fresh name `x:T = zero` (0, 0.0, "", []); after an assignment of x, `int x` stays a conversion.

- P47 lists of tasks: user: "Jobs and tasks are as[ynchronous] by definition if someone waits for one result that
  should not affect the others". So a started job runs on its own; putting it in a list (`jobs.add(j)`) or awaiting
  another job never waits for it, only a use that needs this job's value does. The recommended default fits:
  awaiting only where a value is needed, `await all jobs` for a list.

- P46 type-name and condition patterns in `for` (`for friend in [foe1, friend1, …]`, `for (it>2) in xs`): user:
  "this should give a warning, though if the user is unfamiliar, he needs to confirm that he understands the filter".
  So the wiki forms filter as written, with an educate_once "got it" warning naming the filter (the user confirms
  with got-it; notes/welcoming.md). Done (branch p46-filter-loops, warp_parser try_parse_for_in): `for friend in xs`
  with a declared class (instance_of, the item is `it` and `friend` in the body), `for (it>2) in xs`; got-it topic
  `for-filter`. Built-in type words filter too (`for number in xs`, under their own name; a literal list of matching
  items needs no filter) and an adjective with a type word is a condition (`for (even number) in xs`: even/odd built
  in, else the user's function), branch p46-type-word-filters.
- P61 `x is <value>` with a new name x (old test `x is 100 times [0]`): user: "educate the user to use the be key
  word for definitions". `is` stays a comparison; with an undefined x the warning/error teaches `x be <value>`
  (wiki/be.md, an alias of `:=`, parsed since 67c405a5: `x be 3`, `x be number 3`). The test edit to
  `x = 100 times [0]` (Fixer, branch decided-test-edits) stands, or uses `be` once it parses. Asked by warp-2d.
  Follow-up (user, multiple choice): the typed form too. `x is number 9` with an undefined x no longer declares
  (fix-is-declaration is undone); it teaches `x be number 9` (or `x:number = 9`); `is` is always a comparison.

- P44 re-asked: user: "make shared and atomic synonyms". `atomic xs = int[n]` is the same as `shared xs = int[n]`;
  the rest stays (atomic `+=`/`-=`, `go f(xs)` passes the same array).
- P60 catch handlers: user: "later, but also at the Classical track catch and try except syntax synonyms". The
  function-level handlers (`catch (no food){…}`, `on error{…}`, Error.md) come later; now the classical
  `try {…} catch {…}` (with `catch e`) and Python's `try: … except: …` become synonyms of `try X else Y`.
  Asked by the Fixer warp-2d.

## Decided 2026-10-04 (user; moved out of the pending queue 2026-10-05)
- P31 DECIDED (user, 2026-10-04): the Printable operation is `text(p:person)`, the one allowed exception to type words as function names; `as text`, print and interpolation call it. Printable trait: the operation that gives an instance's text for interpolation, `as text` and print. `text` is a
  type word (P20 forbids it as a function name). Options: `show(p:person)` (Haskell) / `description(p:person)` (Swift)
  / allow `text(p:person)` as the one exception. Not implemented yet (todo.md "Traits: Printable"). Night 2026-10-04.
- P32 DECIDED (user, 2026-10-04): `print xs` of a list prints the str(xs) text; the pinned test in tests/welcoming/test_welcoming_print.rs may change for it. `print xs` of a list variable: allow it with the text `str(xs)` gives ("[1 2]", nested lists too) / keep the error
  "print of a List has no runtime text yet" that tests/welcoming/test_welcoming_print.rs pins. Recommended: allow
  (the text exists now); needs the edit of that pinned test. Night 2026-10-04.
- P33 DECIDED (user, 2026-10-04): REAL threads for `go` (WASM threads with shared memory natively, Web Workers in the browser); last, as a big project: notes/threads.md first. Real concurrency for `go`: tasks now finish where they start (one thread), so pause/stop handlers never run (a
  warning). Options: keep it / wasm threads + shared memory natively and Web Workers in the browser / an event loop
  with explicit yields (`await` points). Assumed: keep it. Night 2026-10-04.
- P34 DECIDED (user, 2026-10-04): a number subscript on an empty `{}` keys it as a map (`d={}; d[1]="a"` → {1:"a"}). `d = {}; d[1] = "a"`: a number subscript of an empty `{}` indexes it as a list (index out of range) / keys it as a
  map like a text subscript does (Lua tables, JS objects). Assumed: list indexing (today's behaviour, a loud error).
  Night 2026-10-04.
- P36 DECIDED (user, 2026-10-04): implement unit conversion as proposed below. Unit conversion: `100 cm in m`, `2 h in minutes`, `3 km as m` are undefined today (`in` places a time in a zone,
  `as` casts to a type). Proposed default: `quantity in unit` and `quantity as unit` give the quantity in that unit of
  the same dimension, exact when it divides (`100 cm in m` → `1 m`), else a ratio (`150 cm in m` → `3/2 m`); the unit
  words take their long names too (minute(s), hour(s), meter(s)); a unit of another dimension is the DimensionError.
  Not implemented (not in the wiki). Night 2026-10-04.
- P37 DECIDED (user, 2026-10-04): named arguments f(name=value)/f(name:value) set parameters and free variables of the body; no implicit capture of same-named variables, a missing argument stays an error. Implemented (src/lowering/named_arguments.rs): `name:{…}` stays an ad-hoc instance (it parses like `name{…}`), so an object passed by name is written `name={…}`; `s:shape` in a signature is a declaration. Gap filling (wiki/gap-filling.md, binding.md, inventions.md): `f y := y*y+v; f(y=2, v=3)` → 7, `fun={x*y};
  fun(x:2 y:3)` → 6, `x=7; f(x):=x*x; f()` → 49: named arguments bind a function's free variables, and a missing
  argument takes the variable of its name. Proposed default: named arguments `f(name=value)`/`f(name:value)` may set
  parameters and free variables of the body; a missing argument stays an error (implicit capture of a same-named
  variable is too surprising). Not implemented. Night 2026-10-04.
- P38 DECIDED (user, 2026-10-04): keep the current default. Variable listeners (wiki/signal.md Todo: "shall event listeners be registrable post-hoc?"): `once x==5 {…}` and
  `whenever cond {…}` are implemented (src/lowering/variable_signals.rs) as checks after each later write of a variable
  the condition reads, in the statements after the listener and their loop bodies; writes before it, in functions
  called later, or in an outer block are not seen, and the condition holding when the listener is declared does not
  fire it. Alternatives: hoist listeners to the top of their block (post-hoc registration), or watch writes in called
  functions too (needs global flags). `on set x {…}` (value = the new value) works the same way, and `after tested:` /
  `before test {…}` (a defined function, also named in the past tense) run after/before each later statement that
  calls it, once per statement even if it calls it twice. `during` is not implemented. Night 2026-10-04.
- P39 DECIDED (user, 2026-10-04): neither `id` nor `square` becomes a builtin; the 5 tests stay ignored ("user: no id/square builtins"). Builtins `id` and `square` (C++ wasp's test runtime words): five ignored tests use them (test_comparison_id,
  test_comparison_id_precedence, test_wasm_function_calls, test_wasm_stuff, test_squares); braceless user functions
  already bind the same way (`f 3+4`). Should they be global library words (a user definition winning), given `square`
  is also a shape type in the trait tests? Parked for the user. Night 2026-10-04.
- P40 DECIDED (user, 2026-10-04): size = count; `byte_size` gives bytes; test_array_constructor stays ignored (P40). `size` of a typed array: test_array_constructor (ignored) wants `size(640000*int)` = 2560000 bytes, the passing
  test_array_length says `size` is a synonym for count. Which is right (bytes as `byte_size`?)? Parked for the user.
- P41 DECIDED (user, 2026-10-04): no separator, as implemented. Juxtaposed print arguments: `print "x changed to " value` prints the parts joined without a separator ("x changed
  to 3"), only when the first part is a text literal (`print first xs` stays a call); commas still join with a space.
  Implemented default (wiki/signal.md example). Alternative: join with a space like the comma form. Night 2026-10-04.
- P42 DECIDED (user, 2026-10-04): keep the parse (comma looser than ==) and warn strongly when a tuple is compared to a bare comma expression `(…) == 2.0, 4`, suggesting parentheses. Comma against `==`: `(2 as float, 4.3 as int) == 2.0, 4` (ignored test_emit_cast_tuple) wants the right side
  to be the tuple (2.0, 4); today the comma binds looser than `==`, so it is `((…) == 2.0), 4`. Tuples in parentheses
  compare element by element now (`(…) == (2.0, 4)` → 1). Change the precedence (or only for a tuple on the left)?
  Not changed. Night 2026-10-04.
- P43 DECIDED (user, 2026-10-04): lists do not grow when an item is set past the end, the empty list included;
  setting past the end is a loud, catchable `index out of range` error, as the existing tests have it
  (`x=(1 2 3);x#4=0`, test_footguns). An experimental growth (night 2026-10-04) was dropped before commit; the
  ignored tests expecting growth (test_array_creation) stay ignored.

## Decided by the user (2026-09-29) — implementation: notes/cloud_tasks.md
- #1 yes: avoid panics everywhere, errors as values. #2 leave the trailers. #3 juxtaposition yes, spaced only if the unit exists.
- #4 `size` = count; bytes via `byte count` / `number of bytes` / `#bytes in list`. #5 units yes.
- #6 data as scope yes, warning when the kebab parts are also variables. #7 unresolved calls are errors.
- #8 print gets an IO capability that eval grants implicitly (hidden). #9 `list of int`, plural type words (`numbers`) are lists.
- #10 no opinion (parked). #11 exact reals may be assigned to a declared float with precision loss: `float x = π` allowed.
- #12 `use <file>` yes. #13 web host later. #14 not-implemented errors unless easy (shifts: implement). Rest: cleanup.

## Decided 2026-09-30
- Keep: unit sums use the finer unit (3010 m); `f - x` with a parameterized user function is `f(-x)`; untyped parameters take
  the kind all call sites agree on, else a loud error.
- Assignment past the end of a list stays an error. `x : 100 int` AND `pixel:int[100]` declare typed arrays.
- A while loop's value is its last body value. `pixels size` (property word after a name) works like `size of pixels`.
- Both `list<int>` and `list of int` in code. Delete test_paint_wasm. Vendor refresh automated (free, only on Cargo.lock change).

## Decided 2026-10-03 (relayed by BOSS-cheeky-shannon to the fixer, playground `print greeting*2 print(g, g) print g, g`)
- `print a    print b` on one line: "Error with hint". Loud error "two statements on one line? separate them with `;`
  or a newline" (warp_parser.rs grouped_list, tests/parser/test_one_line_statements.rs).
- text * number: first "Always ask", then superseded (Asks are being replaced by got-it warnings, warp-b8): repeat the
  text, with an educate_once "got it" warning naming `n times text` (and `int("5")*3` for a number-like text).
  Confirmed by the user as "Python repeat" (P1 below); implemented on fix-text-repeat (tests/text/test_text_repeat.rs).
  text * float and text * text stay type errors; `n times "ab"` is the explicit repeat. This replaces "`"5"*3` stays a
  type error" from 2026-10-02.

## Decided 2026-10-03 (user, multiple choice; not implemented yet)
- Non-digit characters (P27, asked by warp-d2), user to warp-d2, verbatim: "use ord ordinal codepoint() to get the
  code point". So `ord(c)` / `ordinal(c)` / `codepoint(c)` give the code point; `c as int` / `int(c)` of a non-digit
  character is invalid_number, digits stay digits. Revises #35's `'A' as int` → 65. Branch int-of-char edits
  test_text_casts and test_cast_bugs accordingly (user decision).
- int of a character (P25, asked by warp-d2): user to warp-d2, verbatim: "obviously one of five is five". `int('5')`
  is 5 (main does this since a13b4fd6; pinned by tests/text/test_character_comparison.rs, branch int-of-char).
- Text quotes in printed output (asked by warp-6c; user WIP commit ec968e18 expected single quotes in
  tests/test_method_words.rs): "Keep double quotes". The printer keeps `"HELLO"`; the WIP test edits are reverted to
  double quotes (user decision) and the 5 ignored tests un-ignored.
- `try` syntax (P21, asked by warp-bc, branch claude/try-exits-and-naming), user verbatim: "try X else
  otherValueOrAction     I never invented the => Y syntax". The form is `try X else Y`, Y a value or an action;
  the named binding `else e => Y` is not warp syntax and is removed (the question how e binds is moot).
- Objects as arguments (asked by warp-bc for Cloud-Microsoft, branch claude/object-arguments). CENTRAL PHILOSOPHY,
  to be written into the wiki and implemented. User, verbatim: "if it's truly unknown then this is a duct typing like a
  python if it has all the fields but is a known different type then we should create an error and get the user about
  a new keyword image like photo so the image will be treated like a photo and judged by users we should write that as
  a very central philosophy which needs to be implemented". So: a value of truly unknown type passed to `p:photo` is
  duck typed (judged by the fields it is used with); a value of a KNOWN different type (an `image` with all of photo's
  fields) is an error that teaches `image like photo`, which declares image usable as a photo, judged by uses.
  Keyword: "`image like photo`" (chosen over `as`/`is`). Written map `{width:7}` for `p:photo`: "Accept
  structurally". `p.word` typo on an untyped parameter: "Field lookup, runtime error" ("no field word").
- libm FFI (warp-12 experiment: headers serve all of libm on macOS incl. hypot; glibc's __MATHCALL macros defeat the
  header parser on Linux): "Headers first, table fallback". Header-driven linking always; LIBM_UNARY/LIBM_BINARY link
  only when the headers give nothing for "m". Patch: warp.worktrees.noindex/ffi-libm-experiment.patch.
- tests/ folders (asked by warp-1a, branch tests-tidy, notes/tests_layout.md): "OK as listed" (17 topic folders,
  welcoming one folder). Duplicates: "Only probes condense". Probe lines condense into topic files; assertions in
  regular test files stay even when duplicated.
- Wiki remote `main` (asked by warp-d0, notes/wiki_branch.md): "Delete remote main". GitHub wikis serve only master;
  master stays the only branch, agents push `HEAD:master`. The hook blocks agents, so the user runs
  `git -C /Users/me/dev/angles/warp/wiki push origin --delete main`.
- #10 Polish notation for .wat/.wast (P18): "Keep parked". test_wast stays ignored.
- #16 commented `"a".s() + 2` lines in tests/text/test_string.rs (P19): "Delete the lines" (approved test-file edit).
- tests/ layout (P14, code quality 3), verbatim: "The official policy was that probes are can be turned into a real
  tests by condensing that what really matters. We should start a new agent to sort that all out and just grouped into
  folders". A new agent condenses the `probe_*.rs` files into real tests (keeping what matters) and groups tests/ into
  topic folders.
- CLAUDE.md (P15, code quality 8): "Symlink to AGENTS.md".
- Root clutter (P16, code quality 9): "Move to notes/OLD" (the dangling `warp`/`warp` links are deleted).
- D10 return-type polymorphism (P17): "Dispatch on return type" (un-parked). `render "hello" as pdf` /
  `docx example = render "x"` pick the overload by the expected type; ambiguous → got-it warning (assumed under the
  Asks-become-warnings rule).
- smarty.rs + tests/test_asts.rs (P10, code quality 2): "Delete both" (with smarty's asserts in tests/numbers/test_angle.rs).
- Wisp format (P11, code quality 4): "Keep + add a roundtrip test".
- GC reading API (P12, code quality 5): "GcObject". The gc_traits wrappers go.
- libm table (P13, code quality 6): "Keep the table", then verbatim: "Mark the FFI deliberately S. examples and maybe
  quickly try if it works without the hard coding". So `LIBM_UNARY`/`LIBM_BINARY` get a comment saying they are deliberate
  examples of hand-linked FFI, and an experiment checks whether the header-driven FFI serves the same calls without
  them (result decides whether they become example-only).
- Type-word shadowing (P20, asked by warp-bc, branch claude/type-word-user-function): "Clash error". A user function
  named like a type word (`double := it*2`) is an error: "double is a type; rename your function". Reverts the
  shadow-with-warning default.
- cdylib (P7, code quality 10), verbatim: "what is that it doesn't mean that didn't need that before can we gate it".
  Answer: the cdylib is the browser-wasm output, and web/playground/build.sh already asks for it itself
  (`cargo rustc --crate-type cdylib`), so it is already gated. Cargo.toml keeps only `rlib` (since 2024-02 it listed
  `["cdylib", "rlib"]`); the per-worktree crate-type patch is no longer needed.
- vendor/ (P8), verbatim: "currently we don't need it but maybe we want to run an off-line agent later again so let's
  just note that it's currently deactivated". The docs (Cargo.toml, AGENTS.md) say vendoring is deactivated for now;
  `--offline` builds from the registry cache.
- Stale C++ feature flags (P9, code quality 1): "Remove them". The `#[cfg]` branches in tests/wasm/test_wasm.rs and
  tests/web/test_web.rs go, keeping the branch that runs today (approved test edit).
- Asks become got-it warnings (user to BOSS-cheeky-shannon, verbatim): "I really love the got it mechanism for the
  warnings, the Ask mechanism is not what I expected. I thought it would rewrite the code to whatever the user pics we
  don't want context, sensitive execution, lol instead turn all the Ask into a warning with the got it feature plus an
  extra feature for later as an intelligent intent to change the code." Assigned to warp-b8 (ask-to-warning).
- Error-fallback Asks (`[x]*n`, `insert(0, 4)` order, `[1 2 3]+4`, bare `a=1 2 3`, `1+2 squared`): "Stay loud errors".
  They name the explicit forms, no default reading is taken. Asked by warp-b8; its assumption stands.
- Text * number (P1, asked by warp-35): first "actually, why not use the python app approach? Is it really a foot gun",
  then "Python repeat". `"ab"*2` → "abab"; digit text `"5"*3` → "555" with a got-it warning that it is text, not 15.
  Replaces the type error with the repeat hint (also the earlier "text * number: Always ask").
- Typed lists follow-up (P4, asked by warp-5e): "Yes, continue". Float typed arrays, then whole-op Sum/Map ListOps.
- D9 fallback (P5): "Error". An unanswered `1+2 squared` is an error naming both groupings (assumption stands).
- D1 `$` (P6), verbatim: "sorry, I don't know what a hole means but only the one with the curly braces must interpolate
  the other is text like dollar money". So `"${expr}"` interpolates, bare `"$x"` stays the literal text `$x`.
  Revises D1's "also `$x`": tests/text/test_interpolation.rs `dollar_holes_interpolate_too` follows (user decision). Swift
  `"\(expr)"` was not asked about and stays.
- Interpolation holes (user, via the supervisor 2026-10-09, card brace-hole): all four interpolate, `\(…)` canonical,
  `\{…}`, `${…}` and `$(…)` with the hint "prefer \(…)"; `\u{…}` stays a unicode escape.
- Tuple returns (P2, asked by warp-d7/warp-5e): user "yes" (answer "no yes ?" in warp-5e's session, second item).
  `return a, b` and `x, y = f()`, compiled to wasm multi-value without allocating a list. Not built yet.
- Closure Int->Int fast path (P3, asked by warp-5e): user "no" (same answer, first item). Closure calls keep boxing.
- D7 / #33 closures (SUPERSEDED 2026-10-05 by wiki/charged.md: late binding, `global` for changing variables):
  "By value + educate". Blocks keep capturing by value (`x=1; inc:={x=x+1}; do inc; x` → 1); a block
  that assigns an outer variable gets a hint: use `global x` or return the value. The wiki's lazy `:=` examples get updated.
- D3 `[1 2 3]+4`: "Ask". Like `[x]*n`: append or add to each element? Fallback Error. `.+` is element-wise,
  `xs + [4]` concatenates.
- D12 `a=1 2 3` / `a=1,2,3`: first "idk", then "Ask on bare forms": an unbracketed `a=1,2,3` / `a=1 2 3` asks list or
  separate statements, fallback Error; bracketed lists never ask.
- #39 chained comparison: "== never chains". `a<b<c` chains as before, but `1<2==2` is `(1<2)==2` (wiki/Footguns.md
  "Chained comparison" and test_chained_comparison's `1<2==2` line need updating; ask before editing that test).
- D1 interpolation: "Both". Double-quoted text accepts Swift `"\(expr)"` and `"${expr}"` (also `$x`); the normalizer
  picks one canonical form. Single quotes stay literal. Open detail: `$` holes in sql/sh templates must keep working.
- D6 pipe: "The pipe operator is just an operator that behaves differently with different types unified". So `|` is one
  operator dispatched on its operand types: truth values → logical or, a value and a function → pipe (`2|square|root`).
- D14 `x in list`: "The position one index would behave as a truth but maybe it's a foot gun let's try it with a
  warning". `3 in [1 2 3]` gives the position (truthy when found), with a warning. Caveat for implementation: a 0-based
  position makes the first element falsy, so the position must be 1-based (or found-at-0 still truthy).
- #38 `[ø]`: "Truthy". A non-empty list is truthy; the test_wasm_logic_on_objects expectation may be edited.
- D16 numbers: "Unbounded + FFI types". Int stays unbounded (BigInt promotion); `byte`, `int8..int64` exist as declared
  types for FFI that trap on overflow. No `overflow` value.
- D2 `!` (evaluate / mutate / await): "let's think about this later", parked.
- D13 `1 -1`: "ask and assume list". An Ask (signed operand glued after a space) whose default reading is the list
  `[1 -1]` (fallback Warning, taking the list); the arithmetic reading is written `1 - 1`.
- D9 suffix precedence (`1+2 squared`): "Ask". An ungrouped mix asks `1+(2 squared)` or `(1+2) squared`.
- D5 matching by type name: "General rule". Any noun can name a type/parameter (wiki matching.md); open detail: the
  rule for unknown words (`photo`) and multi-word class names.
- D4 constructor vs data: "Distinguish". `T{…}` with a known type constructs/validates, `k:{…}` is plain data, not equal.
- D8 `≈` / `~` / `circa`: "Relative 1e-9 + override". Default relative tolerance 1e-9, settable via `tolerance = …`.
- D15 auto-imports: "Later", parked. D10 return-type polymorphism: parked then, un-parked later as P17 ("Dispatch on return type", done: notes/dispatch.md, src/lowering/overloads.rs).
- #25 / #36 `first [10, 5]`, `reduce [7] …`: "if by subscript you mean index then we already have a rule that space
  disabled index". So a space before `[` never indexes; reading these as an index is a bug against that rule:
  `word [..]` passes the list as an argument, only glued `a[..]` indexes.
- #22 loops: "N once, keep rest". `N times {…}` evaluates N once; trailing `i++ while c` stays a plain while (with a
  hint); `a = 2 if c` keeps guarding the whole assignment.
- #21 type(): "x=π stays real" (bug: `x=π; type(x)` must be real like `type(π)`). Not chosen: `2.0` stays int
  (current), no `rat` alias.
- #27: "Accept Unicode + `be`". `≤ ≥ ≠ × ÷ ¬ √` are accepted alternatives normalized to ASCII; `be` is implemented
  as `:=` (wiki/be.md).
- #26 library: "Extend all". Unicode upper/lower, sort any comparable values, reverse works on text.
- #23: "Warn on `it` shadowing". A loop's `it` still shadows a function's `it`, with a warning.
  #31 (switch message shows value; `min`/`d` units): neither option chosen, stays open.
- #35: "Lists join to "[1 2]"" (the tests/text/test_cast_to_string.rs assertion may be edited; a general runtime serializer
  later) and ""x" as float is loud" (error with a hint; only single-quote codepoints convert to numbers).
- #34 deep traps under `try`: "Later". #37 first-class functions by specialisation: "Enough for now".
- #14e: `download <url>` is an alias of `fetch`.
- #31: "Show runtime value, min/d stay non-units". `no case for n = 4`; `min` stays the function, `d` free.
- #14b test_wasm expectation defects: "Yes, minimal edits" (tolerance compares, expectations per current decisions,
  each edit listed in the commit).
- #19 / #32 stashes: "Drop them if they contain nothing valuable otherwise merge". Checked: concat-wip-tag, the global
  compound-assignment autostash and the README/Footguns autostash were already on main; pre-history-rewrite held only
  .DS_Store and test_results.txt. All 4 dropped. The one unmerged line, the user's 2026-09-27 note on chained
  comparison "BUT NOT WITH == etc !!", became #39 below.
- D4 fields (asked again 2026-10-03, wiki/constructor.md says optional + extendible): "Strict". A missing field is an
  error unless `?` or defaulted (`!` = explicit required); an undeclared field is an error with the hint `T:{…}`.
- Meta information on objects (user idea 2026-10-03: "Think of a special syntax to extend known objects with meta
  information"): existing `@name(value) X` prefix and `X["@name"]` stay; chosen additions: `@key` inside a literal
  (`point{x:1 y:2 @source:"gps"}`, never a field, not counted, ignored by == and by strict validation), `x.@key`
  read/write, glued postfix `x@key`, and attributes survive wasm emission/eval. User: "Can we also access them
  normally as a fallback? point.source first, check if it's a real attribute, then check if it's a meta. Or is that
  too complicated and overkill?" Answer (warp-43): not overkill, one fallback in the field lookup: field first, then
  meta; when both exist the field wins and a hint names `.@source`; neither → the strict "has no field" error.
- Round 3 (2026-10-03, multiple choice):
  - #40 `x -1`: "Keep subtracting". Only a number literal on the left asks (`1 -1`); variables, calls, words subtract.
  - D2 `!`: "By position". `fn!` after a function/method mutates in place (`x.upper!`), `{…}!` evaluates a block,
    await gets its own word `await job` (no `job!`).
  - D14 `in` warning: "Never warn". `3 in [1 2 3]` gives the position silently (the impl-sem warning goes).
  - D5 type-name matching rule (notes/matching.md): "Accept".
  - D15 auto-imports: "Yes, folder scope" (REVISED the same day: opt-in `use folder/package/project`, see above).
  - Meta keys in iteration: "Skip @ keys". Iteration, keys, values and count ignore `@` entries.
  - #34 deep traps under `try`: "Do it now".
  - 14d test_comments2: user "what? 0.length() ???"; explained: the test parses two lines, `y=0` is one Key whose
    length is 0 (C++ counted the 3-item list `[y = 0]`). Correct for the current model, closed.
- D15 REVISED (user to warp-b8, 2026-10-03): "revision of my previous decision create special keyword use folder and
  use package and use Project to automatically include everything but not by default anymore". No automatic folder
  scope; `use folder` (the program's folder), `use package` (below the nearest folder holding <name>.warp),
  `use project` (below the nearest .git), lazy per-name lookup, same conflict rules.
- #30 type tests: "Only `is` tests types". `3 is int` → 1, `3 is rational` → 1; `3 == int` educates toward `is`.

## Decided 2026-10-02 (relayed by warp-f3): eat newcomer syntax, compile its intent, hint the warp form
- Text + number concatenates, the number in its text form (`"F:" + 13` → `"F:13"`, `"5"+3` → `"53"`, JS/Kotlin), with a
  hint `str(13)`. Reversed: the 2026-09 "no implicit conversion" rule (DESIGN.md "Dangerous implicitness", wiki/Footguns.md
  "String + number"), which was there because one-character strings used to add as code points (`"5"+3` → 56). `"5"*3`
  stays a type error. Flipped tests: test_text_concat::text_plus_number_stays_an_error,
  test_footguns::test_text_plus_number_is_a_type_error, test_text_bytes::text_plus_number_stays_a_type_error.
  Not yet: a runtime ratio (`y=2.5; "x"+y`, also `y as string`) prints garbage: list_join has no text form for ratios.
- Extended 2026-10-10 (user via warp-supervisor, card print-oldest): text + anything concatenates in its str() form
  (`"oldest first: " + list`, an instance, a map, ø); a function stays an error. The one exception: a text spelling a
  number plus a number (`"3"+3`) warns, offering `int("3") + 3` or `"3" + str(3)`, and keeps `"33"` (an error under
  `use strict`); `"a"+3` is silent. Test: tests/text/test_text_plus_anything.rs. User, same day: "text + Formatable
  should work in general": an instance of a Printable type (`text(p:P) := …`, a text() method) joins as its own text
  (`"hi " + bo`, `s + bo`; tests/text/test_text_plus_printable.rs); numbers keep joining, `"3"+3` keeps its warning.
- `//` not followed by a space is Python floor division (`7//2`, `a //b`, `x//=3`, `x //= 3`), the Euclidean
  quotient that goes with `%` (`floor_quotient(a, b)`: floor for a positive divisor, `-7//-2` gives 4 where Python
  gives 3). `a div b` is the same floor division. An index that divides (`xs[n/2]`) traps `index must be an
  integer` unless the division is exact, with the hint `n//2`.
- `//` is a comment only when followed by a space or the end of the line: `a // b`, `x = 1 // note` are comments
  (user decision 2026-10-08, emphatic, via the supervisor; it replaces the 2026-10-03 rule that keyed on the space
  before `//`). A comment that hides a closing bracket (`(col // 3)`) is a parse error naming the `//`. A `//`
  comment after code on its line educates once (topic `slash-comment`): "`// …` after code is a comment; floor
  division is written without a space after it: a //b".
- #28 decided (supervisor warp-f3 under the welcoming policy, reported to the user): `x=ø; x.size` and `xs=[]; xs.count`
  are 0; arithmetic on ø still needs the check. Changed line: tests/welcoming/test_footguns.rs test_null_needs_a_check
  (`x=ø; x.size` → 0); tests/lists/test_empty_list_count.rs un-ignored.
- `#` directly followed by a non-space starts an expression (count): `#s`, `#a-1`, `#f(x)`, also at line start
  (fix-sugar-4; before only `#name` as a whole statement counted). Comments: `# text` (space or tab), `#!` (shebang),
  `##` (doc comment) and the directives in warp_parser.rs HASH_DIRECTIVES: `#use`, `#include`, `#import`.
  Side effect: commented-out code written `#code` in samples (samples/raylib_*.warp `#while(1>0){`, `#sleep(2000)`,
  samples/main.warp `#print …`, `#fun …`, samples/lib.warp `#fun ok(){`, tests/warp/ffi/*/*.warp) is now live code
  when run; test_all_samples still parses all 72 samples.
- `let x = …` / `var x = …` declare a variable in any block. USER DECISION (fix-sugar-2): `let` is immutable as
  wiki/variable.md says: `let x=1; x=2` (also `+=`, `++`, `x#i=`) → "x is let (immutable), cannot assign it again;
  fix: declare it with var or plain `x =` if it changes" (check_constants, like const). `var` stays mutable. The `let`
  style hint carries the education "in warp `let` is immutable (unlike JS) …" (one hint, test_normalization pins one).
  fix-sugar-3: the note is `diagnostic::educate_once("let", …)`: shown once per run until the user acknowledges it,
  then remembered as `ack:let` (.warp-answers) and never shown again.
- A bare word statement that names nothing (`x=1; foo; x`, `foo x = 3`) is `undefined variable: foo` (was silently dropped).
- `len(x)` counts like `#x` (hint `#x`); `n times [x]` fills a list; `b=[]; b.count` is 0.
- Rule (user, via warp-f3): newcomer forms are eaten only where they do not clash with a known footgun (text + number
  is the one exception). So, revised in fix-sugar-2:
  - `[x]*n` / `n*[x]` / `[1 2]*2` are refused (wiki/Footguns.md "Lists and arithmetic": Python repeats, NumPy multiplies):
    "ambiguous: Python repeats the list, NumPy multiplies each element; write `n times [x]` to repeat, or map to multiply".
  - `xs.insert(a, b)` never guesses the order (Footguns "Guessing intent"): `insert(x, at: i)` names the position;
    otherwise the kinds decide (the one Int is the position, `insert(0, "z")`, `insert("z", 1)`); two Ints
    (`insert(0, 4)`, `insert(i, v)`) are an error listing both readings, `insert(v, at: i)` / `insert(i, at: v)`.
    So the ignored warp test form `pixel.insert(4,0)` is refused too. Positions are 0-based slots, past the end or
    negative appends. `xs.insert(v)` appends.
  - fix-sugar-3: both are Asks with fallback Error (src/diagnostic.rs). Topic `list-times` (analyzer::lower_list_times,
    a list literal times a number): answers "repeat the list" → `n times [x]`, "multiply each element" →
    `[x].map(x => x*n)`; a list variable times a number stays the plain type error. Topic `insert-order`
    (list_emitter insert_position_and_value, two Int arguments): "position first, as Python" / "value first, as warp".
    Unanswered (tests, CI, pipes): "<question> (too ambiguous to guess); fix: <both explicit forms>". The list-times
    question starts with "type error: list * number:" so test_footguns' `[1 2 3]*2` → "type error" still holds.
  Not yet: `insert 4 at 0`, `at end/start/head`, `x is 100 times [0]` (`is` compares).


## Decided 2026-10-02
- `upto` excludes the end as wiki/range.md says (`1 upto 10` = 1..9); every `upto` hints the explicit forms
  (`..<`/`..` exclusive, `to`/`...` inclusive). tests/control/test_loop_forms.rs `upto_excludes_the_end_unlike_to` follows.

## Original questions
## Blocking finished work
1. **should_panic test** `tests/welcoming/test_footguns.rs:56-60` pins the old compiler panic on undefined variables.
   B9(2) turns all 9 panics into `Error('undefined variable: a')`. Replace with `fails_with("a+1", "undefined variable: a")`?
   (work saved in probes/b9_part2.patch)
2. **Claude-Session trailers** in 27 pushed supervisor commits (forbidden by global CLAUDE.md): rewrite history or leave?

## Language design
3. **Juxtaposition** `3x` → `3*x` (wiki/number.md specifies it; experiment: 0 regressions, makes test_implicit_multiplication pass):
   `1/2x` = `1/(2x)` (Julia) or `(1/2)x`? ordinals `2nd` stay text? `2i` / `2e` complex / constant or product? spaced `2 km` once units exist?
4. **Lists (B8)**: `size` of a list = bytes (wiki/Footguns.md decision) or element count? assigning past the end = error (DECIDED, user 2026-10-04, see P43).
   typed array declarations `x : 100 int`, `pixel:int[100]`, `640000*int`? value of a `while` loop (C++: 0, Rust test: 11)?
5. **Units**: `1 m + 1km`, `1950 ± 50`, `1900 - 2000 AD` (wiki/unit.md) — unit values as identifiers, so `3km` = `3*km`?
6. **Data as scope**: does `a-b:2 c-d:4 a-b` resolve the symbol to its key's value (2)?
7. **Unresolved calls**: `print(3)`, `square(3.0)` without a definition silently become data `(print 3)`. Error in code position?
   Survey (notes/unresolved_call_survey.md): rule `name(` without space in emitted code → `undefined function: name`, data stays data;
   0 test changes. Sub-decisions: `P(1)` type constructor, `x(3)` variable callee (error or multiplication), add `min`/`max`/`print`?
8. **print / I/O under eval**: a print builtin needs an I/O capability, which eval does not grant.
9. **Types**: `type([1 2 3])` → `list` or generic `list<int>`? Should `Data(Vec<i32>)` equal a List of ints?
10. **Polish notation for .wat/.wast**: `(module a b)` → node `module` with children, as a ParserOptions mode (test_wast).
11. **C-style declaration blocks** `double S1 = …, S2 = …` and float literals expected equal to truncated ints (test_primitive_types).
12. **`use <file>`** module import (test_sinus_warp_import).
13. **Web host (L1)**: `$b.ok` / externref need a real webview host (C++ WebApp.cpp; Rust: wry?).

14a. **Shift operators**: `2 << 1` silently gives 0 (parsed as `<` + angle group), `8 >> 1` a cryptic error.
    Add exact-Int `<<` / `>>`, or make the parser reject them loudly?

## Test defects (can't pass unedited)
14b. DECIDED 2026-10-03 (minimal edits). test_wasm expectation defects (exact float compares 4.00001, 2.9999999999999996; `i=123.4;i` → 123; ø expected 0;
    object truthiness; text+text concat) — list in A14 slice 1 report, notes/todo_sweep_task.md A14.
14. RESOLVED (near! in tests/numbers/test_math.rs). `test_sin`: `eq!(sin(pi), 0.)` exact float compare — add tolerance or delete?
14c. RESOLVED (line commented out). `test_named_data_sections` ends with `exit(0)` (tests/wasm/test_wasm.rs:1447): kills the whole test process silently. Remove the line?
14d. CLOSED 2026-10-03 (correct for the Key model). `test_comments2` asserts `(y=0).length() == 3` (C++ model: a 3-item list); in Rust `y=0` is a Key whose length is its value's → 0. Change the expectation?
14e. DECIDED 2026-10-03 (alias of fetch). `download <url>` was never implemented (only `fetch`); add as an alias of fetch?
15. DECIDED 2026-09-30 (delete). `test_paint_wasm`: `w` never assigned, `(x-c)` is a kebab name — edit or delete?
16. C4 `"a".s() + 2` commented lines in test_string.rs — parked (user: don't care).

## Housekeeping (blocked by the destructive-git hook)
17. DONE. Delete duplicate test files `tests/test_footgun_application.rs`, `tests/test_footgun_list_index_bounds.rs` (approved; hook blocked).
18. DONE. Remove scratch worktrees probes/review_wt, probes/float_trunc_wt, probes/wt_before, probes/stage_check,
    ../warp-semicolon-survey; delete probes/review_target, probes/head_check; agent helper scripts in probes/*.py.
19. DECIDED 2026-10-03 (dropped). Old `stash@{0}: autostash` (2026-09-27, README.md + test_results.txt) — keep or drop?
20. RESOLVED for AGENTS.md (CLAUDE.md: pending P15). CLAUDE.md / AGENTS.md describe `src/wit_emitter.rs`, which does not exist.

## New questions 2026-09-30 (supervisor warp-e0), none blocking
Wiki survey: the 16 questions D1–D16 are in notes/wiki_features.md section 2 (D6 `|`/`&` as pipe, D7 lazy `:=` and
D16 overflow contradict Decided rules). Found while implementing:
21. DECIDED 2026-10-03 (see the top). `type(2.0)` is `int` (a decimal with zero fraction normalizes to an integer), `type(1.5f)` float, `type(π)` real,
    but `x=π; type(x)` still float. OK? Add `rat` as an abbreviation of `rational`?
22. DECIDED 2026-10-03 (see the top). `N times {…}` re-evaluates N each round (it reuses the for loop); trailing `while` is a plain while, not do-while
    (`i=5; i++ while i<3` never runs); `a = 2 if c` guards the whole assignment. Keep?
23. DECIDED 2026-10-03 (warn). `for 1..4 {x+=it}` binds `it`; inside a function with an implicit `it` parameter the loop shadows it. Keep?
24. OBSOLETE (upto excludes the end, decided 2026-10-02). `upto` is a global infix word (= inclusive `to`), not only inside `for`. OK?
25. DECIDED 2026-10-03 (see the top: a space never indexes). `first [10, 5]` parses as the subscript `first[10, 5]` (a space before `[` still subscripts); `first [10 5]` works.
    Should a known prefix word followed by a space make `[…]` its argument?
26. DECIDED 2026-10-03 (extend all). Library words: upper/lower are ASCII only (error otherwise), sort ints only, reverse of a text is an error.
    Extend to Unicode / all comparable values? In-place `x.upper!` (wiki D2) not added.
27. DECIDED 2026-10-03 (Unicode + be). Leftovers from earlier sessions: Unicode operators (≤ ≥ ≠ × ÷ ¬ √) and `is` for `==`: canonical or alternatives?
    `be` for `:=` (wiki/be.md) is not accepted by the parser: implement or drop?
28. DECIDED 2026-10-02 (see the top: size of ø is 0). `xs=[]; xs.size` should be 0, but `[]` and `ø` parse to the same node, and tests/welcoming/test_footguns.rs:634
    (`fails_with("x=ø; x.size", "fix: if x {")`, test_null_needs_a_check) pins the null-check error. Change or remove
    that assertion line? The fix (a small arm in check_null_use) is ready; tests/lists/test_empty_list_count.rs waits #[ignore]d.
29. OBSOLETE (checkout now only behind origin). The main checkout /Users/me/dev/angles/warp is diverged: 1 local commit 6c6e9559 (a duplicate of 8bb31618, from
    warp-3f) and behind origin/main; `git merge origin/main` refuses because your staged notes/OLD/* files collide with
    files that came in from origin. Please commit or unstage them, then resolve (the local commit can be dropped).
30. DECIDED 2026-10-03 (see the top: only `is` tests types). `is` and `==` are the same operator, so a type word on the right of `==` is now also a type test (`3 == int` → 1).
    `3 is rational` → 1 (int is a special case of rational). Keep both?
31. DECIDED 2026-10-03 (see the top). `switch n {…}` without a match reports `no case for n` (the subject as written, not its runtime value 4).
    Units: no `min`/`d` (clash with the function `min`), `s`/`h` are now unit words. OK?
32. DECIDED 2026-10-03 (dropped). Stash entries left by workers (the hook blocks `git stash drop`): `stash@{0}` "On text-concat: concat-wip-tag"
    (83a40987, content is merged) plus the older autostash entries. Drop them?
33. DECIDED 2026-10-03 (see the top: by value + educate). A block function that assigns an outer variable does not change it (`inc:={x=x+1}; do inc; x` → 1), because
    closures capture by value (Decided). Should zero-parameter blocks run in the caller's scope instead?
34. DECIDED 2026-10-03 (now, round 3). `try X else Y` catches Error values and the traps directly under `try` (index, /, %, rem); a trap deeper inside X
    (`try 1 + [1 2]#5 else 0`) still ends the program. Full catching needs a host import that runs the guarded body.
    Worth it?
35. DECIDED 2026-10-03 (see the top). `x=[1 2]; x as string` stays a loud error because tests/text/test_cast_to_string.rs (added today by a worker) pins it;
    a join-based "[1 2]" for int lists is ready, a general runtime serializer would be the real fix. Allow editing
    that assertion? Also `"x" as float` → 120 (character code, like `'A' as int` → 65): OK or loud?
36. DECIDED 2026-10-03 (see 25). Parser: `reduce [7] (a b)->a+b` and `first [10, 5]` read `word [..]` as a subscript (see 25).
37. DECIDED 2026-10-03 (enough for now). First-class functions (row 22) are compile-time specialisation (`apply(double2, 3)` → a copy `apply__double2`),
    not a funcref table: functions chosen at run time and capturing lambdas remain loud errors. A table needs one
    uniform (boxed) signature. Enough for now?
38. DECIDED 2026-10-03 (see the top: truthy). Is a list containing only ø falsy (`not ({[ø]})` → true, test_wasm_logic_on_objects)? Today a list is truthy
    when it has a first element.
39. DECIDED 2026-10-03 (== never chains). Chained comparison and `==` (user's note from a dropped 2026-09-27 stash on wiki/Footguns.md: "Intended: mathematical
    chaining as in Python, `3>2>1` → `true`. BUT NOT WITH == etc !!"). Today all comparisons share one level, so
    `1<2==2` chains (`1<2 and 2==2` → true). Should `==`/`!=` stop chaining with `<`/`>`?
40. (impl-ask, 2026-10-03; assumption taken, not blocking) D13 `1 -1` asks only when the left operand is a number
    literal (`1 -1`, `1 +1`, `[1 -1]`); `x -1`, `abs -3`, `return -1` keep subtracting/negating, so
    tests/operators/test_negated_call.rs:10 `is!("x=5;x -1", 4)` stays. Should `x -1` ask too (that test line would change)?
    Also assumed: D9's suffix-precedence Ask falls back to Error.
41. (samples, 2026-10-03; assumption taken, not blocking) Spaced construction: `V { x: 1, y: 2 }` with a space now
    constructs a declared type `V` exactly like the glued `V{x: 1 y: 2}` (D4); a word that names no declared type
    before a block stays data (`Person { name: "Alice" }`). Also: `v.x` reads the field at run time when `x` is a field
    of any declared type, even if v's type is unknown at compile time (a missing field fails "no field x").
    OK, or should the spaced form stay data?
