# warp as a web framework

Roadmap card: web-framework (board topic; its sub-cards are the steps below).

Goal: replace React / Svelte / Next with warp itself: markup is warp data, reactivity is warp's
signals, the server is the same program. The steps are the web-* cards on the board; this note is their order, what
each builds on, and how each works. Defaults marked so are undoable.

## Foundations
- **Markup as data**: `html{ body{ h1: "Hi" ul{ li: "a" li: "b" } } }` parses (samples/web/html.warp, html_dsl.warp; repeated
  keys like `li li li`) and renders as DOM in the page, as HTML natively (lib/markup.warp, step 1 below).
- **Signals** (notes/signals.md): every variable is a signal once something listens; `:=` derives one and knows what it
  reads; `whenever`, `on change`, page-wide `on click` / `on key` in the playground
  and in `warp app` (`on key { exit }` closes it, samples/web/hello_div.warp).
- **Page plumbing** (notes/web_playground.md): the compiler runs in a worker, host words reach the page
  (host.js), paint draws pixels, `use draw` shapes (lib/draw.warp), fetch, channels, components, threads.

## Order of the cards
Each step is useful on its own and is what the next ones stand on.

| # | card | builds on | what it adds |
|---|---|---|---|
| 1 | web-dom (web-dom-render) | markup as data | an `html{…}` / `div{…}` value renders as real DOM in the page and as an HTML string natively (`warp run page.warp > page.html`): tags, attributes (`a{href:"/"}`), text, nesting, escaping |
| 2 | web-element (web-element-events) | 1, signals phase 7 | `button{ on click { count += 1 } "Add" }`: handlers on an element, `event` its data, delegated under the hood |
| 3 | web-fine (web-fine-grained) | 1, 2, derived signals | markup reading a signal (`p{ "total " + total }`, `div{ class: done ? "done" : "open" }`) updates only that text node or attribute when the signal changes; no virtual DOM |
| 4 | web-components | 3 | components are functions returning markup with props and children (`Card(title:"Hi") { p:"text" }`), own signals per instance, cleanup of their listeners when removed |
| 5 | web-keyed (web-keyed-lists) | 3, 4 | `for todo in todos { li{ key: todo.id … } }` patches inserts, removals and moves by key |
| 6 | web-bind | 3 | two-way form bindings (`input{ bind: name }`), submit as an event with the fields as an object, validation from types |
| 7 | web-styles | 1, 3 | styles as warp data scoped to a component, signals in values, plain CSS out |
| 8 | web-async (web-async-data) | 3, fetch | `users := fetch "/api/users"` as a signal with loading / value / error states, refetch on change, cancel on navigation |
| 9 | web-router | 4, 8 | `route "/users/:id" { UserPage(id) }`, History API links, nested layouts, not-found page |
| 10 | web-server | std json | native server in the same program (below); typed RPC between page and server |
| 11 | web-ssr (web-ssr-hydration) | 1, 3, 10 | render at the server or at build time (`warp build --site`), then attach the signals to the existing DOM |
| 12 | web-stores | 3 | shared stores / context as module-level signals, persisted signals (`stored theme = "dark"`), undo |
| 13 | web-dev (web-dev-loop) | 1–4 | `warp dev page.warp`: hot reload keeping signal state, the playground's error overlay in the app |
| 14 | web-testing | 1, 2 | `render(Counter())`, click "Add", expect "1": headless in the browser suite, natively against the HTML string |
| 15 | web-transitions | 5 | enter / leave / move transitions declared as data |
| 16 | web-a11y (web-a11y-i18n) | 1, 4 | accessible defaults with welcoming warnings (an `img` without `alt`), translations as data |
| 17 | web-bundle | all | size and startup budget against Svelte / Solid, tracked in CI |
| — | web-apis | paint, use draw | the browser platform without glue (canvas, WebGPU, storage, WebSocket…), any time; drawing-frames is its first piece |

## Server and page (card web-server)
- **Routes**: `serve 8080 { get "/api/users" { users } post "/api/users" { … } }`. A route's block value is the response:
  a text as text/plain, any other value as JSON. `request` holds method, path, query and body (the
  body parsed when it is JSON); `status 418` sets the status and `header "name" "value"` adds a header (card
  web-route-header; samples/servers/route_headers.warp). lib/web.warp serves the routes of lib/router.warp over the
  process's C sockets, answering as a WAGI build does.
  A WAGI build (`warp build wagi app.warp`, card site-wagi-build) has no host: there they are lib/wagi.warp wagi_header calls,
  which globals collect for the CGI answer; a set Content-Type replaces the answer's own, a failing route answers 500
  whatever status it set.
- **Plain HTTP from the page**: `fetch "/api/users"` gives the parsed JSON as a warp value (arrays ↔ lists, objects ↔
  maps); a status of 400 or more is an Error value carrying the status and the body text.
- **Typed RPC**: `server def users(min_age) {…}` in the same program (lib/web.warp): the function, answered as
  `POST /rpc/<name>` with the request's body as its argument. `/rpc/` is reserved for this.

## Async data (card web-async)
- `users := fetch "/api/users"` at the main level does not wait: `users` is ø with `users.loading` true and
  `users.error` ø; once the reply is in, `users.loading` is false, then `users.error` (the failure text, "fetch … failed:
  HTTP status 404") or `users` (the parsed JSON, any other body the text) is set, so `on change users {…}` and a page
  whose last line is `users` show it.
- In warp: extensions/fetch_signals.warp, the reply set by lib/net.warp fetch_later; written later, the values are
  varying, so the markup reading them renders anew when the reply comes (the page; the native warp answers at once).
- Live regions (card web-a11y-live): the outermost element whose own content reads a fetched value gets
  aria-live="polite", so a screen reader announces it when it arrives; an element setting aria-live keeps its own.
  Sample samples/web/live_region.warp.
- Cancel on navigation: card fetch-cancel.

## Defaults for step 1 (web-dom), undoable
- A markup value is any key whose name is an HTML tag (`div`, `p`, `ul`, `li`, `a`, `button`, … the HTML element list)
  with a block; `name: "text"` is the element with that text, `name{ key: value … }` attributes for keys that are HTML
  attributes, other children nested in order. Unknown names stay data (no custom elements yet).
- Repeated keys (P176): in a named tag's block, glued `ul{…}` or spaced `ul {…}`, a repeated
  `key: value` is a child (`ul{ li: "First" li: "Second" }` is two li), as repeated elements in XML/HTML; a plain `{…}`
  stays a map whose repeated key is the error "duplicate key", also inside a tag, and so does a declared type's
  constructor `Point{ x: 1 x: 2 }`.
- Natively the value serializes as HTML (`to_html`), text escaped. In the page the program's value, when it is markup, is
  shown as DOM in the output pane instead of its warp text; `show(markup)` places it explicitly.

## Step 6 (web-bind)
- `input{ bind: name }` is `input{ value: name on input { name = event.value } }`; a checkbox or radio binds
  `checked`. In warp (card input-bind-name): the rule in extensions/sugar/events.warp makes the item
  `bind: binding(() => name, entered => { name = entered })`, lib/markup.warp's Binding, which renders as
  value or checked, data-warp-input and, from the value's type unless the element names its type, type=number
  step=1 (int), step=any (other numbers), type=checkbox (bool); an int variable stays an int. `input` is a page event;
  the page sends {value, checked} (a number from a number or range field) and sets a changed field's value/checked
  when the markup comes back (morphElement).
- Boolean attributes (checked, disabled, …) are present or absent (lib/markup.warp; true arrives from a run as 1).
- Tour example "form binding" (`typed` field of examples.js).
- `form{ on submit { … event.fields.name … } }` (card web-submit): `submit` is a page event; the page sends
  {fields}, the form's named fields (a number field's a number, a checkbox's checked), and keeps a form with the
  handler on the page (markup-forms.js, a part a built site ships only when its module has `on·submit`); a form without
  one still goes to its route. Sample samples/web/form_submit.warp.
- `select{ bind: choice }` (card web-bind-select): lib/markup.warp hands a select's value to its options (html_render's
  `chosen`), and the option whose value, else text, equals it is `selected`, from the first render on; later the page
  sets select.value (morphElement). Sample samples/web/select_binding.warp.
- Options from a list (card select-shorthand): lib/markup.warp renders what a select holds besides elements as its
  options (html_options), one `<option>` per value (`select{ bind: fruit; ["apple" "pear"] }`, a list variable
  `select{ fruits }`, plain `select{ "apple" "pear" }`), a pair `"s": "small"` the option `value="s"` labelled small.
  No compiler pass (decision markup-plain-library).
- Samples samples/web/component_binding.warp, samples/web/typed_form.warp.

## Built sites (`warp build --site`, extensions/commands/site.warp, cards blockcompiler-entries, build-site)
- `warp build --site app.warp` (`--site=<folder>`) writes site/ beside the program: app.wasm is the program's whole
  module, index.html its first markup prerendered (`warp run` by the same warp, warp_command) and the page's scripts, the
  folder mirrors the repository (web/playground/ holds the scripts) so file reads resolve against the site's root, and
  served-files.js lists the site's files. Any static host serves it; coi-serviceworker.js gives the isolation.
- One path with the playground: site-worker.js runs app.wasm through driver.js programRan, renders it with
  lib/markup.warp page_render after main, each handler (page_event, the element's data-warp-<event>="n") and each timer
  (timerFired); site.js morphs the HTML in (markup.js morphChildren). A program serving requests (lib/web.warp
  page_serve, the run option serve page) gets the page's address, then each link, back button and form, HTTP text both ways
  (host-routes.js requestText, answerOf, shared with the playground's address bar).
- Not yet: holes, route modules, server replies and `warp deploy`'s Cloudflare Worker for built sites; cards
  build-site-prerender, deploy-self.
- Checked: `test_in_browser.py --examples` BUILT_SITE builds a counter site, loads it and clicks its button.

- Page facts (card html-meta, default A): `html{ lang: "en" title: "Shop" description: "…" meta: { author:
  "Ann" } body{…} }` puts `<title>` and `<meta name content>` into the head (lib/markup.warp html_is_page_fact,
  html_page_fact), into the page's own `head{}` when it has one; a `meta{ charset: "utf-8" }` element stays an element
  (html_meta_attributes).

## Routes (card web-router)
- `route "/users/:id" { UserPage(id) }` (lib/web.warp, lib/router.warp): a page, a GET answering its block's markup as
  HTML, `id` bound to that part of the path; `route "*"` matches any path (the not-found page).
- Typed parameters (card route-typed): `route "/users/:id:int" { p{ "next " + (id + 1) } }`. A parameter declaring
  int, float, text or string matches only a part of that type (lib/router.warp route_fits: int digits, float digits
  with one dot), else the next route or not found. An untyped one stays a run-time value of any kind.
- URLPattern syntax (card route-urlpattern, P188 "stay close to the web"): `:tab?` (optional last part, ø when absent:
  `tab or "main"`), `:path*` / `:path+` (the rest of the path, zero / one or more parts, joined by "/"), `/docs/*` (the
  rest, unnamed). Only the last part may carry ?, * or +. `:id:int` stays the typed form. A parameter's regular
  expression (`:id(\\d+)`) matches any part in lib/router.warp.
- The path is the host word page_path(): natively "/", in a built site location.pathname, in the playground "/" until a
  link is followed (host-routes.js).
- Links: site.js follows same-origin `a[href]` clicks without modifier keys with pushState and shows the page anew;
  popstate does the same. In the playground a click on a link inside the shown markup goes to that page of the
  program (playground.js followLink).
- Focus on route change (card web-i18n): after a link or the back button shows another route, host-routes.js
  focusRoute moves the focus to the route's main heading (`main h1`, else `h1`), else `main`, else the page's root, made
  focusable with tabindex -1, so a screen reader reads the new page.
- Not in warp yet: nested routes with a layout's `outlet`, one index.html (+ 404.html) for every path.

## web-apis: WebGPU
- `gpu_compute(shader, numbers, workgroups)` (host word): a WGSL compute
  shader whose entry point `main` reads and writes the numbers as `array<f32>` at @group(0) @binding(0), dispatched
  over `workgroups` workgroups; the value is the list of floats it left. Tests: test/gpu/.
- Browser: host part web/playground/host-gpu.js (needs host-tasks.js). WebGPU only answers
  asynchronously, so a task Worker (task-worker.js `data.gpu`) asks for the device once, runs the job and writes
  {values} or {error} with writeShared; the program's worker blocks in host-tasks.js readShared (shared by tasks,
  fetches and the GPU). A shader that does not compile fails loudly with its line:column. Without task Workers (a page
  that is not cross-origin isolated) it is a loud error.
- Natively: lib/host/native_host/gpu.warp runs the same shader through wgpu-native by plain FFI.
- Render: `gpu_render(shader, width, height)` runs the WGSL fragment shader `main`
  (`@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f`, `at.xy` the pixel's center) over a
  width×height image and gives its pixels as paint takes them, 0xFFRRGGBB row by row (alpha dropped).
  warp appends the vertex stage (warp_full_image: one triangle covering the image) after the shader, so compile errors
  keep the user's line numbers. samples/graphics/webgpu.warp animates rings (a frame per loop step: paint + sleep).
- Values (uniforms): `gpu_render(shader, w, h, {frame: 3, size: [128, 128]})`: the shader reads the map as `values.frame`
  (f32), `values.size` (vec2f; lists of 2–4 numbers are vec2f…vec4f). warp appends `struct WarpValues {…}` and
  `@group(0) @binding(0) var<uniform> values` after the shader (gpu.warp / host-gpu.js uniform_layout: WGSL alignment, 4 /
  8 / 16 bytes, rows of 16) and binds them through an explicit pipeline layout, so an unread value is no error. A
  frame's values change without interpolating them into the shader text.
- Holes (card shader-holes, P235b amended): `$name` in a shader block is a warp variable, written
  as `values.name` (extensions/shader_blocks.warp), added to the values map of each paint / gpu_render of that block,
  so the value is read at each call; an entry written out wins. Bare WGSL names never capture; `$1` stays.
- Ints: a shader declaring its numbers as array<i32> or array<u32> at @binding(0) gets them as
  32-bit ints and gives back ints; an int beyond 32 bits or a float is a loud error.
- Named arrays (card web-apis-rest): `gpu_compute(shader, {xs: […], ns: […]}, workgroups)` binds
  each array where the shader declares the storage array of that name (`@binding(1) var<storage, read_write> ns:
  array<i32>`, gpu.warp / host-gpu.js storageArrays), its element type as declared, and
  gives back the map of the arrays it left, in the order given. A name the shader does not declare, a declared array
  not given or an empty one is a loud error. The tour's GPU_PAGE check.
- Page canvas (card web-apis-rest): in the playground, `paint(shader, w, h, values)` renders
  straight into the page's canvas: the program's Worker asks the page for a canvas (worker.js gpuCanvas hook), which
  sends the canvas's OffscreenCanvas through a MessageChannel to the GPU's task Worker (host-gpu.js gpuCanvas); each
  frame draws into its GPUCanvasContext, and only a {canvas id} message goes to the page, no pixels. gpu_render still
  gives pixels (the program reads them); built sites still paint shaders through pixels (site-worker.js has no
  gpuCanvas hook).
- Next: GPU vectors: card gpu-vectors, notes/gpu.md.

## Step 13 (web-dev)
- `warp dev app.warp [port]` (lib/dev/command.warp, a warp program the native warp runs as `warp app` runs
  lib/app/command.warp; default port 8008, card dev-page) serves the program's site, as `warp build --site` writes it
  into a fresh folder a version, with web/playground/dev.js put into its index.html, the isolation headers and no
  caching. No watcher: a request of the page or its state finding a .warp file of the program's folder newer than the
  last build builds anew. dev.js polls /warp-dev/state ({version, error}) every 300 ms: a new version
  reloads the page; a failure shows as an overlay (message, `line | source`, caret, fix) while the last good build
  serves on, and the next good build reloads. It opens no browser.
- State across reloads (card dev-build): `warp dev` builds with `--dev`, and the extension dev_values
  (extensions/dev_values.warp, before the rules) keeps each main-level variable the program writes again later
  (`x = …` again, `x += …`, in a handler too): its first assignment reads the dev store, `std_io("store", "load", [name,
  initial, "warp-dev"])`, and an added `on set x {…}` saves each later write there. The dev store is the page's
  sessionStorage (markup.js DEV_STORE, prefix "warp dev "), which a reload keeps. Only a written value is saved, so a
  variable never written takes its new initial value from the edited source; a written one keeps its value (as React
  Fast Refresh does). Check: test_in_browser.py DEV_VALUES (click, reload, the count kept).
- Error marks (card web-dev-marks): dev.js devErrorPieces turns the failure's excerpt (`  2 | source` over
  its carets) into the source line with the failing word underlined as playground.css .marked-error, the message on
  hover. Warnings of a good build are not shown.
