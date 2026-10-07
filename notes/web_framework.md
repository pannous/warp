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
