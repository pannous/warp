# wasp as a web framework (plan, 2026-10-07)

Goal (user, 2026-10-07): replace React / Svelte / Next with wasp itself: markup is wasp data, reactivity is wasp's
signals, the server is the same program. The cards are the web-* cards on the board (column Soon); this note is their
order and what each builds on. Defaults below are undoable; open questions go to the Interviewer.

## What exists already
- **Markup as data**: `html{ body{ h1: "Hi" ul{ li: "a" li: "b" } } }` parses (samples/html.wasp, html_dsl.wasp; repeated
  keys like `li li li` since 49a48ece1, tests/web/test_markup_tags.rs). It is only a value today: nothing renders it.
- **Signals** (notes/signals.md): every variable is a signal once something listens; `:=` derives one and knows what it
  reads; `whenever`, `on change`, page-wide `on click` / `on key` in the playground (phase 7).
- **Page plumbing** (notes/web_playground.md): the compiler runs in a worker, host words reach the page
  (host.js), paint draws pixels, `use draw` shapes (std/draw.wasp), fetch, channels, components, threads.

## Order of the cards
Each step is useful on its own and is what the next ones stand on.

| # | card | builds on | what it adds |
|---|---|---|---|
| 1 | web-dom (web-dom-render) | markup as data | an `html{…}` / `div{…}` value renders as real DOM in the page and as an HTML string natively (`warp run page.wasp > page.html`): tags, attributes (`a{href:"/"}`), text, nesting, escaping |
| 2 | web-element (web-element-events) | 1, signals phase 7 | `button{ on click { count += 1 } "Add" }`: handlers on an element, `event` its data, delegated under the hood |
| 3 | web-fine (web-fine-grained) | 1, 2, derived signals | markup reading a signal (`p{ "total " + total }`, `div{ class: done ? "done" : "open" }`) updates only that text node or attribute when the signal changes; no virtual DOM |
| 4 | web-components | 3 | components are functions returning markup with props and children (`Card(title:"Hi") { p:"text" }`), own signals per instance, cleanup of their listeners when removed |
| 5 | web-keyed (web-keyed-lists) | 3, 4 | `for todo in todos { li{ key: todo.id … } }` patches inserts, removals and moves by key |
| 6 | web-bind | 3 | two-way form bindings (`input{ bind: name }`), submit as an event with the fields as an object, validation from types |
| 7 | web-styles | 1, 3 | styles as wasp data scoped to a component, signals in values, plain CSS out |
| 8 | web-async (web-async-data) | 3, fetch | `users := fetch "/api/users"` as a signal with loading / value / error states, refetch on change, cancel on navigation |
| 9 | web-router | 4, 8 | `route "/users/:id" { UserPage(id) }`, History API links, nested layouts, not-found page |
| 10 | web-server | std json | native server in the same program (below); typed RPC between page and server |
| 11 | web-ssr (web-ssr-hydration) | 1, 3, 10 | render at the server or at build time (`warp build --site`), then attach the signals to the existing DOM |
| 12 | web-stores | 3 | shared stores / context as module-level signals, persisted signals (`stored theme = "dark"`), undo |
| 13 | web-dev (web-dev-loop) | 1–4 | `warp dev page.wasp`: hot reload keeping signal state, the playground's error overlay in the app |
| 14 | web-testing | 1, 2 | `render(Counter())`, click "Add", expect "1": headless in the browser suite, natively against the HTML string |
| 15 | web-transitions | 5 | enter / leave / move transitions declared as data |
| 16 | web-a11y (web-a11y-i18n) | 1, 4 | accessible defaults with welcoming warnings (an `img` without `alt`), translations as data |
| 17 | web-bundle | all | size and startup budget against Svelte / Solid, tracked in CI |
| — | web-apis | paint, use draw | the browser platform without glue (canvas, WebGPU, storage, WebSocket…), any time; drawing-frames is its first piece |

## Server and page (agreed with warp-f0, 2026-10-07; card web-server)
- **Routes**: `serve 8080 { get "/api/users" { users } post "/api/users" { … } }`. A route's block value is the response:
  a text as text/plain, any other value as JSON (std json to_json). `request` holds method, path, query and body (the
  body parsed when it is JSON); `status 404` sets the status; `header "name" "value"` later.
- **Plain HTTP from the page**: `fetch "/api/users"` gives the parsed JSON as a wasp value (arrays ↔ lists, objects ↔
  maps); a status of 400 or more is an Error value carrying the status and the body text.
- **Typed RPC**: `server def users(min_age) {…}` in the same program. The server build exposes it as
  `POST /rpc/<name>` with the arguments as a JSON array and the result as JSON; the page build replaces its body with a
  stub making that call (synchronous in the worker, like fetch). `/rpc/` is reserved for this.

## Async data (card web-async, 2026-10-07)
- `users := fetch "/api/users"` at the main level does not wait: `users` is ø with `users.loading` true and
  `users.error` ø; once the reply is in, `users.loading` is false, then `users.error` (the failure text, "fetch … failed:
  HTTP status 404") or `users` (the parsed JSON, any other body the text) is set, so `on change users {…}` and a page
  whose last line is `users` show it. A fetch with `timeout` keeps the old meaning (fetched on each read).
- A URL reading main-level variables (`"/api/users?page=" + page`) is fetched anew when one changes; a reply of an
  older fetch of the same name is dropped.
- Lowering (src/lowering/fetch_signals.rs): the variables users, users·loading, users·error, the call
  `fetch_start(0, url)` and the handler `on·fetch·0`, which takes `fetch_reply(0)` = [value, error]. Natively
  (src/fetches.rs) a thread fetches and the runtime runs the handler at the next check point (warp-runtime
  system_signals await_ready; `warp run` stays until the reply arrived); in the page a task Worker fetches into shared
  memory, which the running main reads at its check points (sleep, loop starts) as natively, so
  `while users.loading { sleep 5 ms }` ends there too; a reply after main returned runs the handler like a timer's
  (worker.js hooks.arrived), then the page re-renders. Without cross-origin isolation (no task Workers) the reply
  only arrives after main.
- Open: cancel on navigation; the page re-renders only a last line that is a name (event_signals with_output_binding),
  so `if users.loading then "Loading…" else users` does not update yet (branch page-binding); diagnostics name
  users·error.

## Defaults for step 1 (web-dom), undoable
- A markup value is any key whose name is an HTML tag (`div`, `p`, `ul`, `li`, `a`, `button`, … the HTML element list)
  with a block; `name: "text"` is the element with that text, `name{ key: value … }` attributes for keys that are HTML
  attributes, other children nested in order. Unknown names stay data (no custom elements yet).
- Natively the value serializes as HTML (`to_html`), text escaped. In the page the program's value, when it is markup, is
  shown as DOM in the output pane instead of its wasp text; `show(markup)` places it explicitly.

## Step 3 (web-fine), what is done and what is left
- Done (DOM side): after a handler the page receives the whole markup again (the output binding, event_signals.rs) and
  playground.js `morphChildren` changes only the text nodes and attributes that differ, matching nodes by position;
  a node of another kind or tag is replaced. Elements keep their identity, focus, input and scroll state. Tour example
  "fine updates" checks it (`clicks`, `clicked`, `kept` in examples.js, test_in_browser.py --examples).
- Done (compute side, card web-fine-holes): the holes of shown markup are the outermost elements holding a computed
  text, attribute or child directly (src/markup.rs `holes`); each is its own binding `page·hole·<path>` (event_signals.rs),
  the path its element indices from the root, which the fixed elements above it keep stable. After a handler worker.js
  reads only the holes and sends `patches` for those whose HTML changed; playground.js morphs just those elements. The
  value text follows once events pause (50 ms). Falls back to the whole markup when the root itself holds a computed
  part, when the last line is a name (components: `page·markup`), or when a hole fails.
- Left: holes inside components (an instance's elements by instance), text-node granularity (a hole is an element).
- Not yet: an `input`'s value property (setAttribute does not change what the user typed): web-bind.

## Step 4 (web-components), what is done and what is left
- A component is a function returning markup; props are its parameters, positional or named (`Card(title:"Hi")`).
- Children: a block after a call that leaves a parameter without its value is that argument, for any function
  (trailing closure, lowering/ruby_blocks.rs): `Card("Hi") { p:"text" }`, `apply(3) { it*2 }`.
- Own state per instance (lowering/component_state.rs, before element_events): a variable of a component that one of
  its element handlers mentions is the main-level list `Counter·count`, one entry per instance; an instance is the n-th
  call of that component in a render (React's hooks rule, undoable default, question queued with the Interviewer); the
  handler's element carries `data-wasp-instance`, the page passes it as `event.instance`. The program's last line
  becomes the getter `page·markup`, which resets the instance counters before each render.
- Class components (warp-41, branch classes-40, notes/classes.md): a class with render() (aliases view, template,
  build) constructed as an element's child or as the program's value renders through render().
- Left: cleanup of listeners when an instance is removed (onMount/onCleanup), instances that move (web-keyed),
  state of a component read by a handler outside it.

## Step 5 (web-keyed), what is done and what is left
- A comprehension or method call among an element's children gives children: `ul{ h2{"todo"} [li{t} for t in ts] }`,
  `ul{ ts.map(t => li{t}) }` (markup_tags.rs keeps it a `[…]`, which the analyzer and emitter take as an item, not as
  statements to run; analyzer/variables.rs: a statement group in a structure declares its locals).
- `li{ key: todo.id … }` is the attribute data-wasp-key; playground.js morphChildren moves the shown element of that
  key into place instead of rewriting elements by position. Tour example "keyed list" (`keyed` check).
- Not yet: `for t in ts { li{t} }` inside a block (the parser reads `ts { … }` as the tag ts; card markup-for),
  transitions (web-transitions).

## Step 6 (web-bind), what is done and what is left
- `input{ bind: name }` is `input{ value: name on input { name = event.value } }` (element_events.rs); a checkbox or
  radio binds `checked`. `input` is a page event (PAGE_EVENTS); the page sends {value, checked} (a number from a
  number or range field) and sets a changed field's value/checked when the markup comes back (morphElement).
- Boolean attributes (checked, disabled, …) are present or absent (std/markup.wasp; true arrives from a run as 1).
- Tour example "form binding" (`typed` field of examples.js).
- Left: `select{ bind: choice }` (its first render shows the first option), `bind:` inside a component's state
  (component_state.rs sees only handlers), form submit as an event with the fields as an object, validation from types.

## Step 7 (web-styles), what is done and what is left
- `style: { color: theme padding: 8 }` on an element is its inline style; `style{ ".card": { padding: 8 } }` a style
  sheet of rules (std/markup.wasp). Numbers are pixels unless the property has no unit (opacity, z-index, …), camelCase names
  are kebab-case; values read variables, so a handler that changes them restyles through the morph. Tour example styles.
- CSS as CSS (classes-42, src/lowering/style_rules.rs, tests/web/test_style_rules.rs): selectors without quotes,
  one rule per line (`.card {…}`, `ul > li {…}`, `h1, h2 {…}`, `a:hover {…}`, `p.note {…}`, `ul li {…}`, `#main {…}`),
  lengths with units (`8px`, `1.5em`, `50%`, `-2px`) and values of several words (`padding: 8px 4px`, `border: 1px
  solid "red"`; a bare number among several words stays as written: `flex: 1 1 auto`, `margin: 0 auto`). The parser reads `.name` at an atom's start as the symbol `.name`. In a style sheet a blank before
  `.x` / `#x` is the descendant combinator (`#main .x`, `.a .b #c`; parser flag in_style_sheet), and `.a {…} #main {…}`
  are two rules on one line (card web-styles-parser). Limits: `50%` followed by another declaration without `;`, and
  several rules on one line with a comma selector need their own lines.
- Left: scoping a component's sheet to its own elements (a generated class per component), warp-cd.

## Step 14 (web-testing), what is done and what is left (classes-42, warp-06)
- `warp::headless::Page` (src/headless.rs, tests/web/test_headless_pages.rs): `Page::render(code)` runs the program
  and keeps the run; `click("Add")` finds the element showing that text with a `data-wasp-click`, runs its handler
  (`on·click·N·node` with `[{instance}]` for a component instance) and reads `page·value` anew; `type_into(label,
  text)` fires `input` on the field found by placeholder, name or id; `text()` (without style/script), `html()`,
  `markup()`. A missing element is an Err naming the page's HTML.
- Natively the run is a kept wasmtime instance (wasm_reader::run_main_kept); in the browser suite the same tests run
  through the new import warp_host.page_event (host.js pageEventOutcome on the last listening run, as worker.js
  showHandled). Elements and text are read from the rendered HTML (headless.rs `shown`), so html.rs is untouched and a
  later renderer (std/markup.wasp) keeps working.
- The playground examples keep their `clicks` / `typed` / `clicked` checks (test_in_browser.py --examples).
- Tests in wasp (src/page_tests.rs, tests/web/test_page_tests.rs; syntax an undoable default, queued with the
  Interviewer): `test "counter" { render Counter(1); click "Add"; fill "name" with "Ada"; check text is "Addn 2" }`.
  A program with a top-level test block that renders runs its tests host-side on headless pages: the code outside the
  tests is the setup, other lines in a test are its own setup, `check` holds with `text` / `html` bound to the page.
  Its value is `2 tests passed` or an Error naming each failed test, the check and what the page shows. Source text is
  used throughout (Node::serialize drops quotes in places). A headless page compiles for a page (pipeline::for_a_page),
  so its page events draw no warning.

## Step 15 (web-transitions), what is done and what is left
- `li{ transition: fade 200ms }` (lowering/transitions.rs, right after markup_tags) is the attribute
  `data-wasp-transition: "fade 200ms"`: its words are data (not variables), durations normalized to ms, a text taken as
  written; the words end where the children begin. In `style: {…}` transition stays the CSS property.
- The page (markup.js morphChildren, Web Animations API, no CSS): an element with a transition animates in when
  inserted and out before removal (marked data-wasp-leaving, skipped by matching, removed when done; a keyed item gone
  from its list leaves where it stands); a keyed one glides to its new place (FLIP). Kinds fade, scale, slide; default
  fade 200ms ease; any other word is the easing. Nothing animates on the first render or with prefers-reduced-motion.
  Tour example transitions (`animated` check of test_in_browser.py); probes/transitions/leave_check.py.
- Left: separate enter / leave kinds (`enter: slide leave: fade`), custom keyframes as data, `0.3s` (card
  fractional-durations), leaving items still take their space until removed (no absolute positioning while leaving).

## Built sites (card web-ssr, 2026-10-07; split agreed with warp-cd, renderer decided by warp-96)
- `warp build --site app.wasp` writes app-site/ (inline code: site/): index.html, app.wasm and the scripts reader.js,
  host.js, markup.js, site.js, carried in the warp binary (src/site.rs include_str!, one source with the playground).
- One renderer, written in wasp (warp-96: "Wasp is wasm-first"): std/markup.wasp's to_html (`use markup`; not `html`,
  which samples/html.wasp would shadow) is the one renderer: the CLI and the playground render a markup value with it
  too (src/markup.rs to_html runs `use markup; to_html(value)`; src/html.rs is gone). A program
  compiled for a page (pipeline::for_a_page) exports page·html := to_html(page·value) (lowering/page_html.rs); its
  page·value is event_signals' output binding, else the last line when that is an expression (no assignment,
  definition, print, use). `warp build --site` runs main and page·html natively (wasm_reader::read_export_after_main)
  for index.html, so the page reads without JavaScript; the page calls the same export after each handler. Page builds also export the reflection getters
  the browser host needs and draw no "a native run never raises it" warning for page events.
- Hydration (web/playground/site.js): the loader runs app.wasm with host.js in the page; main runs once as it ran at
  build time, so the component instances count alike. The DOM stays; click and input on the root find their element's
  handler (markup.js elementEvent), and after a handler (or a fetch reply) the page morphs (markup.js morphChildren) in
  the HTML of page·html.
- std/markup.wasp cannot tell a square list from a curly one at run time: a list value whose items hold pairs is
  inline CSS (for style) or children (`style{ ".x": {…} }`), any other list the joined attribute value (`class:["a" "b"]`).
- Scoped style sheets (card web-scoped): an element with data-wasp-scope:"Card" prefixes the selectors of the sheets
  inside it with `[data-wasp-scope="Card"] `; src/markup.rs is_style_sheet and SCOPE_ATTRIBUTE serve the lowering.
- Timers in a built page: site.js starts them after main (host.js startTimers, shared with the playground worker),
  each handler morphs the page; a failing handler stops them (probes/site/ticker.wasp).
- A program serving its own page: `warp run app.wasp` with `serve 8080 {…}` and a page as its last line answers GET /
  with the page rendered when the server starts (src/site.rs render, the page build without the serve statement) and
  /app.wasm, /site.js … beside it; a route wins over a file of the same path. The page hydrates and fetches the
  program's own routes (probes/site/served.wasp). Page events draw no "never raises it" warning there. Only program
  files serve a page (the host builds it from modules::program_file).
- Open: the page is rendered once at start (per request later); a relative `fetch "/api/…"` fails during that render
  (no server yet), so the first HTML shows the error state until the page fetches; `server def` bodies still run in the
  page (the RPC stub of the page build is not done).
- Routes (web-router, agreed with warp-cf for web-bundle): `route "/x" { Page() }` lowers to page·route·<N>, the table
  is the export page·routes (paths in order), site.js sets page·path, page·html stays the one export; warp-cf splits
  page·route·<N> into app·<N>.wasm and site.js loads it on first navigation.

## Step 12 (web-stores), what is done and what is left
- Persisted signals: `stored theme = "dark"` (lowering/stored_values.rs, soft keyword) is the variable theme holding the
  value an earlier run kept under its name, else the default; `on change theme` keeps each change. Natively the values
  are JSON in `<program>.stored.json` beside the program (in memory for inline code), in the playground the page's
  localStorage (`wasp stored <name>`): the worker gets them at start and sends each save back (host.js
  STD_ADAPTERS.store, playground.js keepStored). Values cross as JSON (std_adapters, as foreign calls).
- Undo history (lowering/undo_history.rs): a program saying `undo x` or `redo x` keeps x's history: after the first
  main-level assignment of x come the lists `undo_past_x`, `undo_future_x` and an `on change x` listener adding the
  old value (not while undo or redo itself writes x); a new change empties what was undone. `undo`, `redo` and
  `stored` are soft keywords.
- Shared stores: a used module's main-level variables are the program's shared state (`use settings` reads and writes
  its theme), and the program's `on change theme` sees the writes of the module's functions (modules::resolve runs
  before the signal passes, card module-signal-writes). `stored theme = "dark"` in a module is a declaration the
  module contributes: kept in the store of the program that uses it (probes/stores/app.wasp, tests/modules/
  test_module_signals.rs). Context (a value for a subtree of components without props): question with the
  Interviewer; default until then: main-level variables, which every component reads.
