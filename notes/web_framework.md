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
- Left (compute side, card web-fine-holes): the markup is still evaluated whole after each handler. Per-hole updates
  (each signal-reading text or attribute its own derived binding, only the changed ones sent) need the lowering to
  mark the holes; worth it once markup gets large (web-components).
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
- Boolean attributes (checked, disabled, …) are present or absent (html.rs; true arrives from a run as 1).
- Tour example "form binding" (`typed` field of examples.js).
- Left: `select{ bind: choice }` (its first render shows the first option), `bind:` inside a component's state
  (component_state.rs sees only handlers), form submit as an event with the fields as an object, validation from types.

## Step 7 (web-styles), what is done and what is left
- `style: { color: theme padding: 8 }` on an element is its inline style; `style{ ".card": { padding: 8 } }` a style
  sheet of rules (html.rs). Numbers are pixels unless the property has no unit (opacity, z-index, …), camelCase names
  are kebab-case; values read variables, so a handler that changes them restyles through the morph. Tour example styles.
- CSS as CSS (classes-42, src/lowering/style_rules.rs, tests/web/test_style_rules.rs): selectors without quotes,
  one rule per line (`.card {…}`, `ul > li {…}`, `h1, h2 {…}`, `a:hover {…}`, `p.note {…}`, `ul li {…}`, `#main {…}`),
  lengths with units (`8px`, `1.5em`, `50%`, `-2px`) and values of several words (`padding: 8px 4px`, `border: 1px
  solid "red"`). The parser reads `.name` at an atom's start as the symbol `.name`. Limits: the parser drops the blank
  of a descendant class (`#main .x` reads as `#main.x`, quote it); `#id` after a rule on the same line, `50%` followed
  by another declaration without `;`, and several rules on one line with a comma selector need their own lines.
- Left: scoping a component's sheet to its own elements (a generated class per component), warp-cd.

## Step 14 (web-testing), what is done and what is left (classes-42, warp-06)
- `warp::headless::Page` (src/headless.rs, tests/web/test_headless_pages.rs): `Page::render(code)` runs the program
  and keeps the run; `click("Add")` finds the element showing that text with a `data-wasp-click`, runs its handler
  (`on·click·N·node` with `[{instance}]` for a component instance) and reads `page·value` anew; `type_into(label,
  text)` fires `input` on the field found by placeholder, name or id; `text()` (without style/script), `html()`,
  `markup()`. A missing element is an Err naming the page's HTML.
- Natively the run is a kept wasmtime instance (wasm_reader::run_main_kept); in the browser suite the same tests run
  through the new import warp_host.page_event (host.js pageEventOutcome on the last listening run, as worker.js
  showHandled). html.rs gained `elements()` / `text_content()`.
- The playground examples keep their `clicks` / `typed` / `clicked` checks (test_in_browser.py --examples).
- Left: tests written in wasp itself (`test "counter" { page = render(Counter()); click page "Add"; … }`) need a
  run-time handler lookup inside the module; not started.
