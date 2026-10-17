# warp as a web framework

Roadmap card: web-framework (board topic; its sub-cards are the steps below, notes/roadmaps.md).

Goal: replace React / Svelte / Next with warp itself: markup is warp data, reactivity is warp's
signals, the server is the same program. The steps are the web-* cards on the board; this note is their order, what
each builds on, and how each works. Defaults marked so are undoable.

## What exists already
- **Markup as data**: `html{ body{ h1: "Hi" ul{ li: "a" li: "b" } } }` parses (samples/web/html.warp, html_dsl.warp; repeated
  keys like `li li li` since 49a48ece1, tests/web/test_markup_tags.rs). It is only a value today: nothing renders it.
- **Signals** (notes/signals.md): every variable is a signal once something listens; `:=` derives one and knows what it
  reads; `whenever`, `on change`, page-wide `on click` / `on key` in the playground (phase 7)
  and in `warp app` (`on key { exit }` closes it, samples/web/hello_div.warp, tests/web/test_app_keys.rs).
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
  a text as text/plain, any other value as JSON (std json to_json). `request` holds method, path, query and body (the
  body parsed when it is JSON); `status 418` sets the status and `header "name" "value"` adds a header (card web-route-header: lowering/web/serve.rs
  with_answer_headers makes them the host word response_header, `status` as the CGI header Status; natively
  web_server.rs Answer::with_headers; a route the page answers itself ignores them; samples/servers/route_headers.warp).
  A WAGI build (`warp build --wagi`, card wagi-build) has no host: there they are lib/wagi.warp wagi_header calls,
  which globals collect for the CGI answer; a set Content-Type replaces the answer's own, a failing route answers 500
  whatever status it set (tests/web/test_wagi_headers.rs, probes/hosting/wagi_routes.sh /teapot).
- **Plain HTTP from the page**: `fetch "/api/users"` gives the parsed JSON as a warp value (arrays ↔ lists, objects ↔
  maps); a status of 400 or more is an Error value carrying the status and the body text.
- **Typed RPC**: `server def users(min_age) {…}` in the same program. The server build exposes it as
  `POST /rpc/<name>` with the arguments as a JSON array and the result as JSON; the page build replaces its body with a
  stub making that call (synchronous in the worker, like fetch). `/rpc/` is reserved for this.

## Async data (card web-async)
- `users := fetch "/api/users"` at the main level does not wait: `users` is ø with `users.loading` true and
  `users.error` ø; once the reply is in, `users.loading` is false, then `users.error` (the failure text, "fetch … failed:
  HTTP status 404") or `users` (the parsed JSON, any other body the text) is set, so `on change users {…}` and a page
  whose last line is `users` show it. A fetch with `timeout` keeps the old meaning (fetched on each read).
- A URL reading main-level variables (`"/api/users?page=" + page`) is fetched anew when one changes; a reply of an
  older fetch of the same name is dropped.
- Lowering (src/lowering/signals/fetch_signals.rs): the variables users, users·loading, users·error, the call
  `fetch_start(0, url)` and the handler `on·fetch·0`, which takes `fetch_reply(0)` = [value, error]. Natively
  (src/web/fetches.rs) a thread fetches and the runtime runs the handler at the next check point (warp-runtime
  system_signals await_ready; `warp run` stays until the reply arrived); in the page a task Worker fetches into shared
  memory, which the running main reads at its check points (sleep, loop starts) as natively, so
  `while users.loading { sleep 5 ms }` ends there too; a reply after main returned runs the handler like a timer's
  (worker.js hooks.arrived), then the page re-renders. Without cross-origin isolation (no task Workers) the reply
  only arrives after main.
- Cancel on navigation: card fetch-cancel. A last line choosing between read-only values (`if users.loading then
  "Loading…" else users`) re-renders too. Open: diagnostics name users·error.

## Defaults for step 1 (web-dom), undoable
- A markup value is any key whose name is an HTML tag (`div`, `p`, `ul`, `li`, `a`, `button`, … the HTML element list)
  with a block; `name: "text"` is the element with that text, `name{ key: value … }` attributes for keys that are HTML
  attributes, other children nested in order. Unknown names stay data (no custom elements yet).
- Repeated keys (P176): in a named tag's block, glued `ul{…}` or spaced `ul {…}`, a repeated
  `key: value` is a child (`ul{ li: "First" li: "Second" }` is two li), as repeated elements in XML/HTML; a plain `{…}`
  stays a map whose repeated key is the error "duplicate key", also inside a tag, and so does a declared type's
  constructor `Point{ x: 1 x: 2 }`. The parser's one-shot flag tag_block (warp_parser/mod.rs) skips the check for the
  tag's own block only.
- Natively the value serializes as HTML (`to_html`), text escaped. In the page the program's value, when it is markup, is
  shown as DOM in the output pane instead of its warp text; `show(markup)` places it explicitly.

## Step 3 (web-fine)
- DOM side: after a handler the page receives the whole markup again (the output binding, event_signals.rs) and
  playground.js `morphChildren` changes only the text nodes and attributes that differ, matching nodes by position;
  a node of another kind or tag is replaced. Elements keep their identity, focus, input and scroll state. Tour example
  "fine updates" checks it (`clicks`, `clicked`, `kept` in examples.js, test_in_browser.py --examples).
- Compute side, card web-fine-holes: the holes of shown markup are the outermost elements holding a computed
  text, attribute or child directly (src/web/markup.rs `holes`); each is its own binding `page·hole·<path>` (event_signals.rs),
  the path its element indices from the root, which the fixed elements above it keep stable. After a handler worker.js
  reads only the holes and sends `patches` for those whose HTML changed; playground.js morphs just those elements. The
  value text follows once events pause (50 ms). Falls back to the whole markup when the root itself holds a computed
  part, when the last line is a name (components: `page·markup`), or when a hole fails.
- Card web-fine-text-nodes: a computed text standing alone between an element's children
  (`section{ h1{…} button{…} "clicked " + n + " times" }`) is a text hole `page·hole·<path>·text·<n>` (markup.rs
  text_holes, n the child elements before it); worker.js sends its plain text, playground.js showText sets that text
  node only, and the element is no element hole. Only a sum starting with a literal text counts (a bare `n` may be
  markup); a text beside it (one DOM text node), a computed attribute, no child elements, or style/script/option/
  select/textarea keep the element hole. Browser check: test_in_browser.py TEXT_HOLE (CI).
- Card web-fine-components: holes inside components. A call of a function showing markup is that
  markup's element (markup.rs holes_through), so a hole's path runs through the instance's root element
  (`page·hole·2·1`: the second Counter's p). Its value cannot be the element as written (it reads the instance's
  state), so component_state.rs binds each hole to its part of one rendering: `page·holes` renders once (page·markup,
  kept in the global page·shown), each hole is `page·element_at(page·shown, path)` or `page·text_at(…, after)`
  (lib/markup.warp html_element_at, html_text_at). worker.js calls page·holes before reading them. The readers are
  defined by page_html.rs use_markup for a program defining a markup function, before modules::resolve keeps the
  definitions they call. Only a program rendering itself gets them. Element handlers (`on click {…}`), still written in
  a component's markup at that point, are neither children nor computed. Browser check: test_in_browser.py
  COMPONENT_HOLE (CI).
- Card web-fine-rest: the root's own computed texts are text holes (`page·hole·text·<n>`, path
  empty; only a root that is an element hole still means none). Built sites patch holes too: host.js pageHoles /
  holePatches read them (worker.js shares pageHoles), markup.js showPatch / showText show them (playground.js,
  site.js, site-thread.js); after main and a route change the whole page·html still morphs. Checks:
  tests/web/test_site_holes.rs (warp app's web view counts whole morphs of #warp-root: 0), test_text_holes.rs.
- Left (card component-calls): a component call under a loop or a choice stays inside its enclosing element hole;
  its instances' paths depend on the loop's count at run time, so their holes would have to be listed at run time
  (from page·shown) instead of bound by the lowering.
- An `input`'s value property (setAttribute does not change what the user typed): Step 6 (web-bind).

## Step 4 (web-components)
- A component is a function returning markup; props are its parameters, positional or named (`Card(title:"Hi")`).
- Children: a block after a call that leaves a parameter without its value is that argument, for any function
  (trailing closure, lowering/control/blocks.rs lower_trailing_blocks): `Card("Hi") { p:"text" }`, `apply(3) { it*2 }`.
- Own state per instance (lowering/web/component_state.rs, before element_events): a variable of a component that one of
  its element handlers mentions is the main-level list `Counter·count`, one entry per instance; an instance is the n-th
  call of that component in a render (React's hooks rule, an undoable default); the
  handler's element carries `data-warp-instance`, the page passes it as `event.instance`. The program's last line
  becomes the getter `page·markup`, which resets the instance counters before each render.
- Class components (classes-40, notes/classes.md): a class with render() (aliases view, template,
  build) constructed as an element's child or as the program's value renders through render().
- Lifecycle (card web-cleanup): `on mount {…}` runs when an instance first renders; `on cleanup {…}` when a render has
  fewer instances of the component than the one before (the last ones are the ones gone, by the order rule), then
  their state entries are dropped. What a cleanup reads is kept per instance like handler state. Tour example cleanup.
- `(if c then A() else [])` among an element's children is a child (markup_tags.rs), not a statement.
- Outside its component (card web-outside-state, user decision): `counters`, Counter's lower-case plural, is its
  instances in page order; `counters#1.count` is the first one's count (readable and assignable), `counters.count`
  every one's. `Counter.instances` is the long form, with a hint to write `counters` (`Counter.all` would clash with the
  list method all); `Counter.count` stays short for `counters.count`. A program's own variable `counters` stays its
  own. component_state.rs with_state_lists maps them to the state lists `Counter·count`. Sample
  samples/web/component_totals.warp.
- Keyed instances (card web-instance-keys): `Counter(t, key: t.id)` ties an instance's state to its key. A keyed
  component gets the parameter `key = ø` and the list `Counter·keys`; each instance moves its key's entries of the
  render before to its position (a swap), a new key takes a fresh slot (what stood there moves to the end), so after
  the render the state lists are in page order again and truncation, cleanup and `event.instance` work as before;
  mount runs for a new key. Unkeyed calls keep the order rule. Sample samples/web/keyed_counters.warp.

## Step 5 (web-keyed)
- A comprehension or method call among an element's children gives children: `ul{ h2{"todo"} [li{t} for t in ts] }`,
  `ul{ ts.map(t => li{t}) }` (markup_tags.rs keeps it a `[…]`, which the analyzer and emitter take as an item, not as
  statements to run; analyzer/variables.rs: a statement group in a structure declares its locals).
- `li{ key: todo.id … }` is the attribute data-warp-key; playground.js morphChildren moves the shown element of that
  key into place instead of rewriting elements by position. Tour example "keyed list" (`keyed` check).
- A for loop among an element's children, `ul{ for t in ts { li{t} } }`, is the comprehension `[li{t} for t in ts]`
  (markup_tags.rs loop_as_comprehension; in a for header the parser no longer reads `ts { … }` as the child tag ts).
  A spaced element statement `ul { … }` reads as the glued `ul{ … }` (card markup-ul).
- Transitions: Step 15.

## Step 6 (web-bind)
- `input{ bind: name }` is `input{ value: name on input { name = event.value } }` (element_events.rs); a checkbox or
  radio binds `checked`. `input` is a page event (PAGE_EVENTS); the page sends {value, checked} (a number from a
  number or range field) and sets a changed field's value/checked when the markup comes back (morphElement).
- Boolean attributes (checked, disabled, …) are present or absent (lib/markup.warp; true arrives from a run as 1).
- Tour example "form binding" (`typed` field of examples.js).
- `form{ on submit { … event.fields.name … } }` (card web-submit): `submit` is a page event; the page sends
  {fields}, the form's named fields (a number field's a number, a checkbox's checked), and keeps a form with the
  handler on the page (markup-forms.js, a part a built site ships only when its module has `on·submit`); a form without
  one still goes to its route. Headless: `Page::submit(label, fields)`. Sample samples/web/form_submit.warp.
- A handler on a line of its own inside an element (`form{⏎ on submit { … }⏎ button{…} }`) is the element's, not the
  tag `on{submit{…}}` (markup_tags.rs attributes, element_events.rs handler_at).
- `select{ bind: choice }` (card web-bind-select): lib/markup.warp hands a select's value to its options (html_render's
  `chosen`), and the option whose value, else text, equals it is `selected`, from the first render on; later the page
  sets select.value (morphElement). Sample samples/web/select_binding.warp.
- Markup mistakes (card our-html, accessibility.rs markup_mistakes, beside the a11y warnings): a key that is no
  attribute (`hreff:`, shown as text; fix the near attribute, else `data-…`), a handler of no page event (`on clik`), an
  input type the browser does not know (`nubmer`), content of a void element (`img{ "x" }`, dropped). Handlers are taken
  out of an element's items first, so `input{ on input {…} }` no longer reads its event as a nested input.
- Options from a list (card select-shorthand, select_options.rs, a source pass before markup_tags): a list item in a
  select is one `option{}` per value (`select{ bind: fruit; ["apple" "pear"] }`), a map item `"s": "small"` the option
  `option{ value: "s" "small" }`, a list variable `select{ fruits }` the loop `for … in fruits { option{…} }`. A list
  right after the bound variable (`bind: fruit ["apple" "pear"]`) is split off it, no longer glued into the value.
- `bind:` inside a component (card web-bind-component): element_events::lower_binds spells binds out before
  component_state, which then keeps the bound variable per instance like any handler's. Sample
  samples/web/component_binding.warp.
- Validation from declared types (card web-validate): element_events::lower_binds gives a bound field the checks of
  its variable's declared type (`age: int = 30`): int → type number, step 1 (the browser marks 2.5 :invalid and a
  form will not send it); float → number, step any; bool → checkbox. An attribute the element sets wins. The C-style
  `int age = 30` is the same declaration: type_name_matching::lower_prefixed_declarations rewrites it early to
  `age: int = 30`, so handlers also treat it as the main-level variable (card bind-ignores). Not yet: ranges or text
  patterns as types. Sample samples/web/typed_form.warp.

## Step 7 (web-styles)
- `style: { color: theme padding: 8 }` on an element is its inline style; `style{ ".card": { padding: 8 } }` a style
  sheet of rules (lib/markup.warp). Numbers are pixels unless the property has no unit (opacity, z-index, …), camelCase names
  are kebab-case; values read variables, so a handler that changes them restyles through the morph. Tour example styles.
- CSS as CSS (classes-42, src/lowering/web/style_rules.rs, tests/web/test_style_rules.rs): selectors without quotes,
  one rule per line (`.card {…}`, `ul > li {…}`, `h1, h2 {…}`, `a:hover {…}`, `p.note {…}`, `ul li {…}`, `#main {…}`),
  lengths with units (`8px`, `1.5em`, `50%`, `-2px`) and values of several words (`padding: 8px 4px`, `border: 1px
  solid "red"`; a bare number among several words stays as written: `flex: 1 1 auto`, `margin: 0 auto`). The parser reads `.name` at an atom's start as the symbol `.name`. In a style sheet a blank before
  `.x` / `#x` is the descendant combinator (`#main .x`, `.a .b #c`; parser flag in_style_sheet), and `.a {…} #main {…}`
  are two rules on one line (card web-styles-parser). In a style sheet (the flag holds inside its rules too) a hex
  color is CSS's: `#8884`, `#fff`, `#1e1e1e` in a value are texts, not the count or index operator; an id of hex letters
  followed by a block or selector part stays a selector (`#fed { … }`), an id never starts with a digit (card css-block,
  parser css_color_length). `:root {…}` starting a rule is the pseudo-class selector, and a selector before a comma
  continues into the group after it (`h1, :hover {…}`, `ul li, .x {…}`); a sheet's `html {…}` is a rule, no element
  (no accessibility warning; card markup-style-block). A `%` glued to a number is a percentage, also before the
  next declaration on its line (`width: 50% height: 10px`), and rules with comma selectors may share a line (card
  web-styles-limits).
- Scoping a component's sheet to its own elements: done (card web-scoped, under step 15 below).

## Step 14 (web-testing) (classes-42)
- `warp::web::headless::Page` (src/web/headless.rs, tests/web/test_headless_pages.rs): `Page::render(code)` runs the program
  and keeps the run; `click("Add")` finds the element showing that text with a `data-warp-click`, runs its handler
  (`on·click·N·node` with `[{instance}]` for a component instance) and reads `page·value` anew; `type_into(label,
  text)` fires `input` on the field found by placeholder, name or id; `text()` (without style/script), `html()`,
  `markup()`. A missing element is an Err naming the page's HTML.
- Natively the run is a kept wasmtime instance (wasm_reader::run_main_kept); in the browser suite the same tests run
  through the new import warp_host.page_event (host.js pageEventOutcome on the last listening run, as worker.js
  showHandled). Elements and text are read from the rendered HTML (headless.rs `shown`), so html.rs is untouched and a
  later renderer (lib/markup.warp) keeps working.
- The playground examples keep their `clicks` / `typed` / `clicked` checks (test_in_browser.py --examples).
- Tests in warp (src/web/page_tests.rs, tests/web/test_page_tests.rs; syntax an undoable default): `test "counter" { render Counter(1); click "Add"; fill "name" with "Ada"; check text is "Addn 2" }`.
  A program with a top-level test block that renders runs its tests host-side on headless pages: the code outside the
  tests is the setup, other lines in a test are its own setup, `check` holds with `text` / `html` bound to the page.
  Its value is `2 tests passed` or an Error naming each failed test, the check and what the page shows. Source text is
  used throughout (Node::serialize drops quotes in places). A headless page compiles for a page (pipeline::for_a_page),
  so its page events draw no warning.

## Step 15 (web-transitions)
- CSS form (P188, card web-css; lowering/web/transitions.rs, right after markup_tags): `li{ transition: opacity 200ms }` is
  the element's inline CSS transition, joined to its own `style`; `starting-style: { opacity: 0 }` (CSS's
  @starting-style, which an inline style cannot hold) is the attribute `data-warp-starting-style`, rendered as
  declarations by lib/markup.warp. Words are data (not variables), durations normalized to ms, a text taken as written,
  timing words (`ease-out`) join the transition; the words end where the children begin. In `style: {…}` transition
  stays the CSS property as written.
- The kinds before CSS, `transition: fade 200ms` (also scale, slide), lower to that CSS (opacity, and transform for
  scale/slide, plus their starting style) with an advise hint naming the CSS form.
- The page: web/playground/markup-transitions.js, loaded after markup.js only by sites whose module mentions
  "transition" (site.rs scripts_of; playground index.html and pages.yml SITE_FILES list it), replaces markup.js's
  defaults transitionPlaces, enter, leave, moveFrom (hello-world budget stays small). Web Animations API from the
  computed CSS transition: an element with a starting style animates in from it when inserted and towards it before
  removal (marked data-warp-leaving, skipped by matching, removed when done; a keyed item gone from its list leaves
  where it stands); a keyed one whose transition covers transform (or all) glides to its new place (FLIP). Nothing
  animates on the first render or with prefers-reduced-motion. Tests: tests/web/test_css_transitions.rs,
  test_transitions.rs; tour example transitions (`animated` check of test_in_browser.py);
  probes/transitions/leave_check.py.
- Leaving space (card web-leave-space): a leaving item keeps its space while it animates out (no absolute
  positioning); when it is removed, its keyed siblings with a transform transition glide into the space (FLIP in
  markup-transitions.js leave) instead of jumping. Sample samples/web/leaving_list.warp; probes/transitions/leave_check.py
  reports `glided`.
- Keyframes are CSS's own (card web-keyframes, user decision via P188): `style{ "@keyframes pop": { from: {…} "60%":
  {…} to: {…} } }` in the sheet, `animation: pop 400ms` in an element joins its inline style beside a transition.
  What CSS lacks is a removal that waits: `leave: pop 400ms reverse` is the attribute data-warp-leave, the animation
  markup-transitions.js plays before it removes the element (then its siblings glide into the space). The sheet's key
  must be quoted: unquoted `@keyframes pop { from … to … }` reads `from … to` as a range. The invented inline
  `keyframes: [...]` list was dropped. Sample samples/web/keyframes.warp.
- Fractional durations work: `0.3s` is 300ms.
- Scoped (card web-scoped): a component whose markup holds a style sheet names itself on its root element
  (`data-warp-scope="Card"`, component_state.rs) and its sheet's selectors are prefixed with
  `[data-warp-scope="Card"] ` (html.rs), so they style only elements inside it (not the root itself; a nested
  component's elements still match).
- The parser bits (`.card { … }` without quotes, `8px` as a length) are done: step 7 above.

## Built sites (card web-ssr)
- `warp build --site app.warp` writes app-site/ (inline code: site/): index.html, app.wasm and the scripts reader.js,
  host.js, markup.js, site.js, carried in the warp binary (src/web/site.rs include_str!, one source with the playground).
- One renderer, written in warp (warp is wasm-first): lib/markup.warp's to_html (`use markup`; not `html`,
  which samples/web/html.warp would shadow) is the one renderer: the CLI and the playground render a markup value with it
  too (src/web/markup.rs to_html runs `use markup; to_html(value)`; src/html.rs is gone). A program
  compiled for a page (pipeline::for_a_page) exports page·html := to_html(page·value) (lowering/web/page_html.rs); its
  page·value is event_signals' output binding, else the last line when that is an expression (no assignment,
  definition, print, use). `warp build --site` runs main and page·html natively (wasm_reader::read_export_after_main)
  for index.html, so the page reads without JavaScript; the page calls the same export after each handler. Page builds also export the reflection getters
  the browser host needs and draw no "a native run never raises it" warning for page events.
- Hydration (web/playground/site.js): the loader runs app.wasm with host.js in the page; main runs once as it ran at
  build time, so the component instances count alike. The DOM stays; click and input on the root find their element's
  handler (markup.js elementEvent), and after a handler (or a fetch reply) the page morphs (markup.js morphChildren) in
  the HTML of page·html.
- lib/markup.warp cannot tell a square list from a curly one at run time: a list value whose items hold pairs is
  inline CSS (for style) or children (`style{ ".x": {…} }`), any other list the joined attribute value (`class:["a" "b"]`).
- Scoped style sheets (card web-scoped): an element with data-warp-scope:"Card" prefixes the selectors of the sheets
  inside it with `[data-warp-scope="Card"] `; src/web/markup.rs is_style_sheet and SCOPE_ATTRIBUTE serve the lowering.
- Timers in a built page: site.js starts them after main (host.js startTimers, shared with the playground worker),
  each handler morphs the page; a failing handler stops them (probes/site/ticker.warp).
- A program serving its own page: `warp run app.warp` with `serve 8080 {…}` and a page as its last line answers GET /
  with the page rendered when the server starts (src/web/site.rs render, the page build without the serve statement) and
  /app.wasm, /site.js … beside it; a route wins over a file of the same path. The page hydrates and fetches the
  program's own routes (probes/site/served.warp). Page events draw no "never raises it" warning there. Only program
  files serve a page (the host builds it from modules::program_file).
- Since done: a page asking the server is rendered per request with the replies carried along (P221 step 3,
  2da366f5e), the page build's RPC stub (cards rpc-stub, rpc-everywhere; notes/server_routes.md). Left there: a page's
  main-level statement reading server data (card web-page-server-data), access rules (card route-access).
- Programs in a Worker (card site-worker, step 1): a module importing task words (task_/channel_/shared_, site.rs
  runs_in_a_worker) runs in site-worker.js, where a blocking await may wait (Atomics.wait) and its tasks run together on
  the task Workers. The page loads site-thread.js, markup.js and site.js only; its root lists the Worker's scripts
  (data-warp-worker="reader.js,host.js,host-tasks.js…"), and site.js calls startSiteWorker instead of hydrate: the page
  sends the module, kept values, path and element events, the Worker answers html/stored/print/failure. A static host
  gets cross-origin isolation through coi-serviceworker.js (one reload). A plain page stays on the page's thread (bundle
  budget). Routes (step 2, card site-worker-step): the page keeps host-routes.js (links, back button, focus) and sends
  the Worker {navigate: path}; the Worker loads that route's module and answers its markup with `navigated`, after
  which the page focuses the route. Probe: probes/site_worker.py (also probes/site/routed_tasks.warp).
- Paintings (card site-frames): a module importing paint runs in the Worker too (an animation sleeps between frames
  and reads the pointer while it runs). The site ships canvas.js (the playground's paint and pointer code, moved out
  of playground.js): the Worker posts {paint}, site-thread.js draws it into one canvas after the root (outside the
  morphed markup; a new size replaces it) and shares the SharedArrayBuffer pointer for mouse_x/mouse_y/mouse_down.
  Checked natively by test_site a_painting_site_runs_in_a_worker_with_its_canvas; the page side has no browser test.

- Page facts (card html-meta, default A): `html{ lang: "en" title: "Shop" description: "…" meta: { author:
  "Ann" } body{…} }` puts `<title>` and `<meta name content>` into the head (lib/markup.warp html_is_page_fact,
  html_page_fact), into the page's own `head{}` when it has one; a `meta{ charset: "utf-8" }` element stays an element
  (html_meta_attributes). Test: tests/web/test_html_meta.rs; probe probes/html_meta/page_facts.warp.

## Routes (card web-router)
- `route "/users/:id" { UserPage(id) }` (lowering/web/routes.rs, lib/router.warp): each route is the function page·route·N,
  `id` bound by `let` to that part of the path (a number when it is digits, else the text), page·routes gives the
  patterns in order (exported, for the per-route modules), page·routed() is the first route matching the page's
  path, else "no page at <path>"; `route "*"` matches any path (the not-found page). A layout shows page·routed() where
  it says `outlet`; else a program ending with a route shows it as its last line.
- Typed parameters (card route-typed): `route "/users/:id:int" { p{ "next " + (id + 1) } }`. A parameter declaring
  int, float, text or string is `let id:int = route_segment(…) as int`, so its block type-checks; the route matches
  only a part of that type (lib/router.warp route_fits: int digits, float digits with one dot), else the next route or
  not found. An untyped one stays a run-time value of any kind (`"a" + (id + 1)` is a type error: declare the type).
  An unknown type is an error naming the known ones. Casts use `as`: float(text) is broken (card float-of-text), and a
  std function returning int, float or text mixed came back int-typed (card return-type-mixed).
- URLPattern syntax (card route-urlpattern, P188 "stay close to the web"): `:id(\\d+)` (a regular expression the part
  must match whole, written as in a JS string; page·route_index checks it with std regex, imported only then),
  `:tab?` (optional last part, ø when absent: `tab or "main"`), `:path*` / `:path+` (the rest of the path, zero / one
  or more parts, joined by "/"), `/docs/*` (the rest, unnamed). Only the last part may carry ?, * or +; an unnamed
  group `(…)` and a mark before the last part are loud errors. Not supported: `{…}` groups, marks in the middle.
  `:id:int` stays the typed form (a group or mark makes the parameter untyped).
- The path is the host word page_path(): natively "/" (host::with_page_path for a render at another path), in a built
  site location.pathname (site.js hooks.pagePath), in the playground "/" until a link is followed. host.js
  navigate(holder, hooks, path) sets it and calls hooks.navigated (pending fetches are dropped there).
- Links: site.js follows same-origin `a[href]` clicks without modifier keys with pushState and shows page·html anew;
  popstate does the same. The playground sends {navigate: path} to the worker (playground.js followLink), which shows
  page·value, else page·routed. A routes program stays listening for that. Tour example routes.
- Nested routes (card route-nested): `route "/users" { div{ h1{ "Users" } outlet } route "/" {…} route ":id:int" {…} }`.
  Inner patterns are relative to the outer one, the outer block's other items are its layout, showing the inner route
  at `outlet`. Flattened in lowering/web/routes.rs: each inner route is a page route with the whole pattern
  ("/users/:id:int") whose function is the layout with page·part·N() at the outlet (page·part·N: the inner block, its
  parameters, the outer ones too, bound by `let`); after them the outer route itself with an empty outlet, so an inner
  "/" route answers the outer path first. page·routes lists the whole patterns.
- One page for every path (card single-page; the user preferred the single HTML, replacing the
  per-route pages of route-prerender and the per-request rendering of serve-route): a site with routes is index.html
  (rendered at "/") plus its copy 404.html with <base href="/">, which static hosts (GitHub Pages) serve for any path
  they have no file of; site::file_at answers such a path with it in `warp dev` and in a serve program
  (tests/fixtures/served_routes.warp). site.js runs main at location.pathname and shows that route, so a deep link
  shows the home page's HTML until the module ran. The <base> makes the scripts, app.wasm and app-route-N.wasm load
  from the root (a site in a subfolder of its host gets no deep links). Browser check: probes/site/deep_link_in_browser.sh.
- `outlet` is the canonical word (aliases such as slot on demand).

## Step 12 (web-stores)
- Persisted signals: `stored theme = "dark"` (lowering/data/stored_values.rs, soft keyword) is the variable theme holding the
  value an earlier run kept under its name, else the default; `on change theme` keeps each change. Natively the values
  are JSON in `<program>.stored.json` beside the program (in memory for inline code), in the playground the page's
  localStorage (`warp stored <name>`): the worker gets them at start and sends each save back (host.js
  STD_ADAPTERS.store, markup.js keptValues / keepValue, shared with built sites: site.js). Values cross as JSON
  (std_adapters, as foreign calls).
- Runtime keys (card web-apis, storage): `storage` is the same store as a map: `storage[k] = v`, `storage[k]` (ø when
  absent), `delete storage[k]`, `keys(storage)`, `storage.k`; the store words remove and names beside load and save.
  Words (P188, browser API names): `local[k]` is that store
  (localStorage), `storage[k]` its alias; `session[k]` a store of its own (sessionStorage while the tab lasts, natively
  in memory while the process runs; SESSION_STORE "warp-session", host.js sessionValues). A program defining its own
  `storage`, `local` or `session` keeps it.
- IndexedDB: `database[k]` (alias `indexedDB`) is a store of its own for values beyond
  localStorage's ~5 MB, same forms (`database.k`, delete, keys). Natively `<program>.database.json` beside the program
  (inline code: in memory); in the browser the IndexedDB database `warp`, object store `values`, all in host-files.js
  (shipped only with programs keeping values, so a hello-world site stays in its byte budget): loadDatabase reads it
  once before the program runs (worker.js, site-worker.js, site.js; Workers have IndexedDB too), each change is
  written back where the host keeps values (self.keepStored; the browser test suite keeps them in memory). Any store file ending in `database.json` (stored_values.rs DATABASE_STORE) is the
  database in the browser. Checked by hand in the playground across reloads; a built site's path is not yet checked
  in a browser (card web-database-site). Probe: probes/stores/database.warp.
- Clipboard (P188, navigator.clipboard's names): `clipboard.write(text)` is the std word
  std_io("clipboard", "write", [text]) (lowering/signals/system_values.rs): pbcopy / wl-copy / xclip natively, in a page
  markup.js copyText (navigator.clipboard.writeText, which needs a recent click; a refusal goes to the console), from a
  Worker through its page (self.writeClipboard). `clipboard.read()` is `clipboard` (natively pbpaste). In the browser
  (card web-apis-rest) the program's Worker asks its page (host-tasks.js askPage, the word in
  host-tasks.js so only a program reading the clipboard ships it) and waits on shared memory while the page reads
  navigator.clipboard.readText (task-workers.js pasteInto/answerInto); the browser may refuse it without a recent
  click: a loud error. The tour checks it where the browser allows (CLIPBOARD_READ).
- Undo history (lowering/definitions/undo_history.rs): a program saying `undo x` or `redo x` keeps x's history: after the first
  main-level assignment of x come the lists `undo_past_x`, `undo_future_x` and an `on change x` listener adding the
  old value (not while undo or redo itself writes x); a new change empties what was undone. `undo`, `redo` and
  `stored` are soft keywords.
- Shared stores: a used module's main-level variables are the program's shared state (`use settings` reads and writes
  its theme), and the program's `on change theme` sees the writes of the module's functions (modules::resolve runs
  before the signal passes, card module-signal-writes). `stored theme = "dark"` in a module is a declaration the
  module contributes: kept in the store of the program that uses it (probes/stores/app.warp, tests/modules/
  test_module_signals.rs). Context (a value for a subtree of components without props): undoable default
  main-level variables, which every component reads.

## Step 17 (web-bundle)
- Budget: tests/web/test_bundle_budget.rs builds `p{ "hello world" }` as a site and holds its gzipped total under a
  budget, printing each file (the suite's log tracks it), about 20 KB; Svelte's hello world is about 3 KB, Solid's about 5 KB.
  What keeps it small: the Unicode case table as ranges (wasm_emitter/text_unicode.rs), site::compacted (shipped
  scripts without comment lines, indentation and a statement's trailing `; // comment`), host-media.js (paint, text_coverage, int-list helpers), html_kebab
  ASCII-only (the Unicode case table was reached only through html_kebab → lower: app.wasm 12.0 → 10.8), exact.js
  (exact numbers, starts the tasks, foreign and compiler parts) and std.js (stdCall with std_pure/std_io, starts the
  hashes and files parts). Helper files prepended to parts hold function declarations only (a site may load them twice).
  Find growth with integrator_tools/site_size.sh per merge, then scratch tools: gzip per JS block, wasm export →
  function reachability. The budget is 100 KB, so it never blocks a branch; the test still prints sizes.
- Not taken: wasm-opt -Oz on app.wasm (11 KB smaller raw, 0.6 KB larger gzipped).
- What is left in app.wasm is the runtime lib/markup.warp's to_html reaches (texts, lists, equality, floats): a shared
  runtime module, cached across pages, would help sites with several pages, not the first load.
- host.js split: the core keeps what every page runs (run, outcome,
  page events, values between JS and warp); the parts add themselves with addHostPart:
  host-files.js (fetch, read, std file, net, store and os), host-hashes.js (std_pure: hash, json, regex), host-timers.js
  (on every …, at 9:00), host-random.js (random, seeded), host-tasks.js (tasks, channels,
  BroadcastChannel and WebSocket, shared arrays, fetch_start), host-foreign.js (foreign_call, libm, libc.wasm, .wasm
  imports; needs files), host-compiler.js (warpHost, run_block; needs files), host-routes.js (page_path, navigate, a site's links and back button). A part hooks into a run through its
  steps (started, poll, finished, ended, stopped). src/web/site.rs HOST_PARTS ships a part when the module imports one of
  its words (wasmparser); the workers load all (HOST_PART_FILES). tests/web/test_host_parts.rs checks that each word
  a part gives selects it. std modules (card web-host-parts-std): the emitter names the std modules a
  program calls through std_pure/std_io in the custom section warp.std_modules (`*` for one named at run time: every
  part), host_parts.rs reads them as imports of that pseudo-module. host-sound.js (sound_samples, std sound and midi)
  is split from host-files.js, so `stored` values ship no sound code (tests/web/test_host_parts_std.rs).
  Still coarse: json ships the hashes (host-hashes.js, 5.5 KB) and tables stay in host-files.js: a tables part would
  give no host word, which test_host_parts each_part_is_chosen_by_the_words_it_gives allows only for host-hashes.js.
- Lazy loading per route (src/web/route_split.rs, tests/web/test_route_modules.rs, browser probe
  probes/lazy_routes/check_in_browser.sh): `warp build --site` moves each route's function page·route·N and the
  functions only it reaches into app-route-N.wasm with binaryen's `wasm-split --multi-split` (features named one by one,
  src/modules/binaryen.rs: --all-features would emit exact imports no browser takes). app.wasm keeps a table slot and a
  placeholder import (`placeholder.app-route-N`) per moved function. site.js instantiates app.wasm (host.js
  instantiateProgram), loads the module of the route the path picks (host-routes.js loadRouteModule: page·route_index,
  instantiated with app.wasm's exports as `primary`), then runs main (runMain); each navigation loads the next route's
  module once. Without wasm-split on PATH the site ships one module with a note; a dev site never splits.
  User functions only one route reaches move too (card web-split-helpers, tests/web/test_route_helpers.rs):
  the emitter lists them in the custom section `warp.user_functions` (host.rs USER_FUNCTIONS_SECTION); they are
  exported only for tasks and other modules, so route_split.rs leaves them out of the call graph's roots, and
  wasm-split keeps each export as a `trampoline_<name>` thunk through the table. A module spawning tasks (task_spawn,
  task_spawn_values) keeps them all: a task's Worker calls one by name in a fresh instance without the route's module.
  wasm-split runs with --debuginfo, so app.wasm and the route modules keep their name sections.
  Limits: the runtime the module exports stays in app.wasm; data segments stay too. A user function the compiler folded
  away (fib(15) → "610") stays as dead code in app.wasm: nothing drops unreached user functions of a site yet.

## web-apis: animation frames (card drawing-frames, first piece of web-apis)
- In the playground a paint after a `sleep` is an animation's next frame: `loop { clear(paper); …; show(); sleep(16) }`
  shows each frame at once in place of the last canvas (host.js sleep → worker message "sleep" → playground.js
  painted). Frames keep the run alive past RUN_TIMEOUT_MS; editing the code stops the animation (show → stopRun) and
  runs the new code. Paints without a sleep between them stay one canvas each. Tour example animation (`canvases` check).
- Frames draw into the canvas shown (showFrame), so the pointer stays over it. `mouse_x`, `mouse_y` (canvas pixels) and
  `mouse_down` are system values (host_words.rs): the page keeps the pointer over a canvas in a SharedArrayBuffer that
  the worker reads at once, also in the middle of an animation, when it takes no messages (playground.js
  trackPointer → worker pagePointer → host.js system_value). `on click` over the canvas gives event.x / event.y.
  Natively mouse_x is a loud error (no canvas). Tour example mouse.
- Natively each show still writes paint-N.png (src/graphics/paint.rs). `color.with_alpha(a)` works as a method (test_draw.rs).
  Built sites show frames and read the pointer too (card site-frames, "Paintings" under Built sites).

## web-apis: notify
- `notify "text"` is the host word notify (src/host.rs, warp-runtime system_values.rs notify): natively osascript
  `display notification` (macOS, the text as an argument, never quoted into the script) or notify-send (Linux); in the
  playground the page shows the browser's Notification once allowed, and until then, or when refused, the printed line
  `notification: text` (playground.js notification; the first one asks for the permission). Tests check only the
  compiled import (tests/web/test_web_apis.rs): a test run shows no notification.
- Clipboard: Step 12; WebSocket: below.

## web-apis: WebGPU
- `gpu_compute(shader, numbers, workgroups)` (host word, warp-runtime host_words.rs GPU_COMPUTE): a WGSL compute
  shader whose entry point `main` reads and writes the numbers as `array<f32>` at @group(0) @binding(0), dispatched
  over `workgroups` workgroups; the value is the list of floats it left (f32: WGSL has no f64). Example:
  probes/webgpu/double.warp; tests/web/test_webgpu.rs (browser suite: real GPU, skips loudly without an adapter).
- Browser: host part web/playground/host-gpu.js (site.rs HOST_PARTS, needs host-tasks.js). WebGPU only answers
  asynchronously, so a task Worker (task-worker.js `data.gpu`) asks for the device once, runs the job and writes
  {values} or {error} with writeShared; the program's worker blocks in host-tasks.js readShared (shared by tasks,
  fetches and the GPU). A shader that does not compile fails loudly with its line:column. Without task Workers (a page
  that is not cross-origin isolated) it is a loud error. A built site using GPU words or paint (which may be given a
  shader) ships host-gpu.js with host-tasks.js and runs its program in the site Worker (card site-gpu).
- Natively: src/graphics/gpu.rs runs the same shader through wgpu 30 (Metal, Vulkan or DX12; the
  `native` feature; pollster blocks on its futures), one device per process; errors in the browser's form, `1:10:
  expected identifier…` or wgpu's innermost cause. A machine without an adapter says "no WebGPU adapter" (tests skip).
  Cost: 69 more crates in Cargo.lock, a first build of about a minute. Not a sample yet.
- Render (card g_YqWY): `gpu_render(shader, width, height)` runs the WGSL fragment shader `main`
  (`@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f`, `at.xy` the pixel's center) over a
  width×height rgba8unorm image and gives its pixels as paint takes them, 0xFFRRGGBB row by row (alpha dropped).
  warp appends the vertex stage (warp_full_image: one triangle covering the image) after the shader, so compile errors
  keep the user's line numbers. Same task-Worker path as gpu_compute (host-gpu.js GPU_JOBS), natively src/graphics/gpu.rs
  render. samples/graphics/webgpu.warp animates rings (a frame per loop step: paint + sleep), in the playground menu again.
  Without an adapter the samples test skips it loudly (CI).
- Values (uniforms): `gpu_render(shader, w, h, {frame: 3, size: [128, 128]})`: the shader reads the map as `values.frame`
  (f32), `values.size` (vec2f; lists of 2–4 numbers are vec2f…vec4f). warp appends `struct WarpValues {…}` and
  `@group(0) @binding(0) var<uniform> values` after the shader (gpu.rs / host-gpu.js uniform_layout: WGSL alignment, 4 /
  8 / 16 bytes, rows of 16) and binds them through an explicit pipeline layout, so an unread value is no error. Leaving
  the map out passes a null node (ffi_emitter.rs fills a missing argument), i.e. no values. A frame's values change
  without interpolating them into the shader text.
- Holes (card shader-holes, P235b amended): `$name` in a shader block is a warp variable, written
  as `values.name` (the parser, src/graphics/shader_holes.rs shader_text, marks the text with attribute shader_holes). The source
  pass shader_holes::lower adds `name: name` to the values map of each paint / gpu_render of that block or of a variable
  holding it, so the value is read at each call; an entry written out wins (samples/graphics/webgpu.warp: `$size` is the vec2
  given). A values map that is not a literal (a variable) cannot take entries: a loud error naming them. Bare WGSL
  names never capture; `$1` stays. This also answers would-blue: `$blue` is the warp spelling, `values.blue` the WGSL
  it becomes. tests/programs/test_shader_holes.rs.
- Ints: a shader declaring its numbers as array<i32> or array<u32> at @binding(0) gets them as
  32-bit ints and gives back ints (gpu.rs element_of / compute_ints, host-gpu.js gpuElements); an int beyond 32 bits or a
  float is a loud error. tests/web/test_webgpu_ints.rs.
- Named arrays (card web-apis-rest): `gpu_compute(shader, {xs: […], ns: […]}, workgroups)` binds
  each array where the shader declares the storage array of that name (`@binding(1) var<storage, read_write> ns:
  array<i32>`, gpu.rs STORAGE_ARRAY / storage_arrays, host-gpu.js storageArrays), its element type as declared, and
  gives back the map of the arrays it left, in the order given. A name the shader does not declare, a declared array
  not given or an empty one is a loud error. tests/web/test_webgpu_named.rs, the tour's GPU_PAGE check.
- Page canvas (card web-apis-rest): in the playground, `paint(shader, w, h, values)` renders
  straight into the page's canvas: the program's Worker asks the page for a canvas (worker.js gpuCanvas hook), which
  sends the canvas's OffscreenCanvas through a MessageChannel to the GPU's task Worker (host-gpu.js gpuCanvas); each
  frame draws into its GPUCanvasContext, and only a {canvas id} message goes to the page, no pixels. A canvas the
  task Worker does not hold (another took the GPU) is made anew. gpu_render still gives pixels (the program reads
  them); built sites still paint shaders through pixels (site-worker.js has no gpuCanvas hook).
- Next: GPU vectors: card gpu-vectors, notes/gpu.md.

## web-apis: WebIDL
- `use js <global>` of a browser global is typed through WebIDL as `use c` is through C headers: src/formats/web_idl.rs reads
  lib/web.webidl (bundled from w3c/webref's @webref/idl 3.85.0 by scripts/webidl_bundle.py: console, Crypto,
  Performance, Storage, Navigator, Location, Clipboard whole, i.e. with every partial, mixin and parent of all 334 spec
  files; Window and its mixins only with the attributes that name those globals). foreign_modules.rs checks a member of
  such a global at compile time: one the interface lacks (did you mean, case-insensitive first), an attribute called,
  an argument count no overload takes. Natively too: node resolves a name to its global first, which is the same
  object (node-only members such as performance.eventLoopUtilization are refused: `use js "perf_hooks"`).
- A global WebIDL does not declare (Math, JSON: ECMAScript; node modules) stays unchecked. Tests: tests/ffi/test_web_idl.rs.
- Scope (slice 2): natively a program's globals are Window's; without the native feature (the playground, the browser
  suite) WorkerGlobalScope's, where they run: `use js localStorage` says it exists on a page, not in the Worker, and
  names local[k]; `navigator` is a WorkerNavigator (no clipboard). A global is a namespace (console) or an attribute of
  the scope, never an interface name (`use js Storage` is the constructor: unchecked). Member-level [Exposed=Window]
  inside mixins (NavigatorID.vendor) is not read yet.
- Chains (slice 3): an attribute's value is typed by its interface when the bundle declares it whole
  (`navigator.clipboard.writeTxt` → Clipboard, did you mean writeText), also through a variable
  (`board = navigator.clipboard`; foreign_modules.rs web_idl_values). Results of operations stay untyped.
- Results (slice 4): an attribute or operation WebIDL declares as DOMString (USVString…), boolean, an integer or a
  float type is lowered to `foreign_call(…) as text|bool|int|float` (web_idl.rs result_type, PRIMITIVE_TYPES), a warp
  value: `crypto.randomUUID().upper()` is warp's upper, not a JS call. Nullable (`DOMString?`: getItem), undefined,
  objects, promises and overloads with differing returns stay untyped foreign values. Fixed alongside: `(f() as text)`
  / `str(…)` of a foreign call gave the source of the call (emitter mentions_call now counts host::VALUE_GIVING_WORDS).
- Nullable results (card text-generally): `DOMString?` is `text?` (web_idl.rs optional_result_type), lowered by
  src/lowering/types/optional_casts.rs: `x as T?` keeps ø, any other value is cast to T, the call held once in `optional·N`.
  Natively a JS null (getItem of a missing key) arrives as ø (foreign.rs plain), no longer as a handle of null.
- DOM (slice 5): the bundle adds Document, Element, NodeList, HTMLCollection, DOMTokenList, Event and every
  HTML…Element of html.idl (lib/web.webidl ~100 KB; arguments in scripts/webidl_bundle.py's docstring). What an
  operation gives is typed like an attribute (web_idl.rs result_interface: `document.getElementById(id)` is an
  Element, `document.body` an HTMLElement). An interface also takes the members of those deriving from it
  (derived_interfaces: an Element's `value` is HTMLInputElement's), as the runtime element may be any of them; only a
  member none declares is an error, its suggestion drawn from all of them. `style` is CSSStyleProperties, generated
  per CSS property, not bundled: unchecked. Only on a page: the Worker (playground) and node have no document.
- Global object (slice 6): `use js self` / `globalThis` (`window` on a page; in the Worker an error naming self) is
  the program's scope, with WindowOrWorkerGlobalScope bundled whole (fetch, atob, setTimeout…) plus Response and
  Headers. The scope is bundled in part, so a member it lacks stays unchecked (`window.innerWidth`). A `Promise<T>`
  result is T: node's loop awaits a call (src/modules/foreign.rs), so `self.fetch(url)` is a Response, `.text()` a text,
  `.status` an int. Natively node mirrors self/window as globalThis. In the browser host main awaits it via JSPI
  (card jspi-page, notes/web_playground.md "Foreign runtimes"); without JSPI (Safari) a promise is a clear error. warp's own `fetch "/x"` is unaffected.
- P203: `use js <global>` is the annotation (like `use c` headers), so these checks stay compile errors.
- Constructors (slice 7): `use js URL; URL(text)` calls the constructor (JavaScript's `new`: foreign_call member "",
  foreign_modules.rs CONSTRUCTOR_MEMBER; src/modules/foreign.rs and host-foreign.js construct). WebIDL's `constructor(…)`
  members (web_idl.rs Definition.constructors) check the argument count, an interface without one is an error
  (`Location()`), and the value is typed as the interface. Bundled for this: URL, URLSearchParams, Blob, Headers,
  WebSocket. A global WebIDL does not declare (Date) is constructed unchecked.
- Canvas (slice 8): `canvas.getContext("2d")` of a literal id is a CanvasRenderingContext2D (web_idl.rs
  CONTEXT_INTERFACES: webgpu → GPUCanvasContext, webgl/webgl2, bitmaprenderer; an OffscreenCanvas's 2d is
  OffscreenCanvasRenderingContext2D), also of an Element (declared_result looks in the derived interfaces). WebGPU's
  entry points bundled: GPU (navigator.gpu), GPUAdapter, GPUDevice, GPUQueue, GPUCanvasContext; requestAdapter's
  Promise<GPUAdapter?> types as GPUAdapter. lib/web.webidl ~120 KB.
- WebSocket instances stay untyped in practice (native ws:// channels are the WebSocket path); the missing
  JS→warp callbacks (onmessage over the node pipe, promises in the browser host) are card js-callbacks.

## web-apis: WebSocket (card web-websocket)
- No new words: a channel named by a ws:// or wss:// address is a WebSocket. `on message from "wss://…" { … event … }`
  connects at once (natively an unreachable server is a loud error there) and hears what the server sends;
  `broadcast value on "wss://…"` sends over the same connection (or opens one). A text goes as it is, any other value
  as JSON; an arriving JSON object or array is data, any other message a text (web_server.rs value_of_body).
- Natively src/web/web_sockets.rs (tungstenite 0.30 with rustls webpki roots): a thread per connection reads with a 20 ms
  timeout and writes what the program sent; channels.rs / host.rs channel_next ask it first. Listeners on one address
  share its connection. Tests: tests/control/test_web_sockets.rs against a local tungstenite server that answers.
- Playground: host.js webSocket / sendOnSocket, a browser WebSocket per address in the run, closed with the run
  (stopListening); messages are handled when the worker is idle, as for BroadcastChannel channels. Checked by hand
  against wss://echo.websocket.org (text and JSON).

## Step 13 (web-dev)
- `warp dev app.warp [port]` (src/web/dev_server.rs, default port 8008) serves the program's site from memory
  (site::files, the same page `warp build --site` writes, plus web/playground/dev.js). No watcher: a request finding
  the file's mtime changed builds anew. dev.js polls /warp-dev/state ({version, error}) every 300 ms: a new version
  reloads the page; a failure shows as an overlay (message, `line | source`, caret, fix) while the last good build
  serves on, and the next good build reloads.
- State across reloads: a dev build (pipeline::for_dev) keeps each main-level variable the program changes later
  (assigned again or `+=`, stored_values.rs dev_kept) as `stored` keeps it, in the store DEV_STORE: the page's
  sessionStorage (`warp dev <name>`), in memory natively. Only a changed value is saved, so a variable never changed
  takes its new initial value from the edited source; one changed keeps its value (as React Fast Refresh does).
  site.js hydrate loads the kept values and shows what main left (page·html).
  Component instance state too (card web-dev-instances): in a dev build component_state.rs declares each state list
  (`Counter·count`, and a keyed component's keys) from the dev store and saves it after every render, so each instance
  keeps its state across a reload (instances are found by call order or key, as within one run).
- Tests: tests/web/test_dev_server.rs; probes/dev/counter.warp (click, edit the label: the count stays),
  probes/dev/stored_check.py.
- Modules (card web-dev-modules): the loader records the files a build used or included (modules::module_files), and
  dev_server.rs watches them beside the program. A module's parse failure names its file (` at greeting.warp:2:15`,
  diagnostic::located_in_file), so the overlay shows the module's line (host_parts::with_excerpt reads that file).
  A failure past parsing in a module's code (an unknown word) names the file too (card module-error-file): the parser
  marks a module's positions with its file (meta::parsing_file, LineInfo.file), the emitter's errors and Diagnostic
  print `file:line:column` (meta::place_text), and names_position reads that as a position. Probes:
  probes/dev/uses_module.warp with probes/dev/greeting_module.warp, probes/module_error_file/uses_broken.warp.
- Browser (card dev-opens-browser): `warp dev` from a terminal opens its page in the default browser
  (dev_server::serve_and_open, `open` / `xdg-open` / `start`); piped, under WARP_NO_WINDOW or CI it only prints the
  address, and `dev_server::serve` (tests) never opens. Test warp_dev_opens_no_browser_when_piped.
- Error marks (card web-dev-marks): dev.js devErrorPieces turns the failure's excerpt (`  2 | source` over
  its carets) into the source line with the failing word underlined as playground.css .marked-error, the message on
  hover (tests/web/test_dev_marks.rs runs it under node). Warnings of a good build are not shown.

## Step 16 (web-a11y) (classes-44)
- Welcoming warnings over the markup as written (src/web/accessibility.rs, run on the parsed program at the start of
  pipeline lower_for_emission, tests/web/test_accessibility.rs): img without alt (alt:"" marks a decoration); input /
  select / textarea without a label (label{for}, a label around it, aria-label, aria-labelledby or title count; a
  placeholder alone gets its own warning; hidden/submit/button/reset/image inputs need none); button or a with no text
  and no aria-label/title; a without href; a heading skipping a level (h1 → h3); an id used twice; html without lang.
  Each points at the element and names a fix; `use strict` / `--strict` make them errors like every warning.
- Translations as data: `use i18n`, translate(messages, language, key, values) with CLDR plural forms (notes/i18n.md).
- Focus on route change (card web-i18n): after a link or the back button shows another route, host-routes.js
  focusRoute moves the focus to the route's main heading (`main h1`, else `h1`), else `main`, else the page's root, made
  focusable with tabindex -1, so a screen reader reads the new page (probes/lazy_routes/check_in_browser.sh).
- Markup built at run time (card web-a11y-runtime): the markup a run hands back (pipeline checked_markup) goes through the
  same rules; a computed tag comes back as a text (`("h" + level): title` as `"h2": …`) and is named like a tag as
  written. Problems the markup as written already showed are not repeated; the others are runtime warnings
  (`markup built at run time: h3 after h1 skips a heading level; fix: h2`), each once a run. Sample
  samples/web/computed_tags.warp. Not checked: markup only a later page event builds (the playground renders it in its
  module, headless Page re-renders through page·html).
- Live regions (card web-a11y-live): the outermost element whose own content reads a fetched value (`users := fetch …`)
  gets aria-live="polite" (fetch_signals.rs with_live_regions), so a screen reader announces it when it arrives; an
  element setting aria-live keeps its own. Sample samples/web/live_region.warp.
