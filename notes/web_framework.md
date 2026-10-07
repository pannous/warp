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
- A for loop among an element's children, `ul{ for t in ts { li{t} } }`, is the comprehension `[li{t} for t in ts]`
  (markup_tags.rs loop_as_comprehension; in a for header the parser no longer reads `ts { … }` as the child tag ts).
  A spaced element statement `ul { … }` reads as the glued `ul{ … }` (card markup-ul).
- Not yet: transitions (web-transitions).

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
- Left: scoping a component's sheet to its own elements (a generated class per component), `.card { … }` written
  without quotes (the parser stops at `.`), `8px` written as a number with a unit (parses as 8 * px).

## Built sites (card web-ssr, 2026-10-07; split agreed with warp-cd, renderer decided by warp-96)
- `warp build --site app.wasp` writes app-site/ (inline code: site/): index.html, app.wasm and the scripts reader.js,
  host.js, markup.js, site.js, carried in the warp binary (src/site.rs include_str!, one source with the playground).
  index.html holds the program's value as HTML, rendered at build time (server-side rendering): the page reads
  without JavaScript. Pages are compiled with the reflection getters the browser host needs (pipeline::for_a_page),
  and their page event handlers draw no "a native run never raises it" warning.
- Hydration (web/playground/site.js): the loader runs app.wasm with host.js in the page; main runs once as it ran at
  build time, so the component instances count alike. The DOM stays; click and input on the root find their element's
  handler (markup.js elementEvent), and after a handler the page morphs (markup.js morphChildren) the HTML the program
  renders itself.
- One renderer (warp-96: "Wasp is wasm-first"): the HTML of the live updates comes from the program, an export
  page·html that the emitter builds, reusing how print renders Nodes inside wasm; no second renderer in JS, no
  compiler shipped with the site. Once page·html exists, the build-time rendering runs that same export natively and
  src/html.rs no longer renders sites. Until then a built page is static after load (site.js warns on the console).
- Open: page·html (next step); timers and fetch replies in a built page (site.js has the hooks, no timer loop yet);
  `serve` programs serving their own page.

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
- Shared stores: a used module's main-level variables are already shared (`use settings` reads and writes its theme),
  but a program's `on change theme` misses writes made by the module's functions: card module-signal-writes
  (probes/stores/app.wasp). Context (a value for a subtree of components without props): question with the
  Interviewer; default until then: main-level variables, which every component reads.
