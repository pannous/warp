# One source for the server and its page

Roadmap card: web-framework (board topic; its sub-cards are the steps and leftovers below, notes/roadmaps.md).

One source file is compiled twice:

| build | what it makes of | made by |
|---|---|---|
| server (native, `warp run app.warp`) | `serve 8080 { get "/api/x" {…} }` serves HTTP, `server def f` is also POST /rpc/f, the page's site answers GET / and any other path | lowering/web/serve.rs, web_server.rs, site::served_files |
| page (wasm in the browser, pipeline::for_a_page) | `route` patterns run in the page's router, `serve` is left out, each call of a `server def f` is a value fetched from POST /rpc/f | lowering/web/routes.rs, serve.rs without_serving, site.js |

Example (probes/server_routes/one_source.warp):

```warp
users = ["Ann", "Bo"]
server def user_count() { count(users) }
serve 18461 { get "/api/users" { users } }
route "/" { div{ h1{ "Users" } a{ href:"/users/1" "the second" } } }
route "/users/:id:int" { p{ "User " + users#id } }
route "*" { p{ "no such page" } }
```

GET /api/users gives `["Ann","Bo"]`, POST /rpc/user_count gives `2`, GET /users/1 is the one page (index.html,
`<base href="/">`) whose router shows "User Ann" once hydrated; a link from / navigates without a reload.
`route` (pages) and `get` (data) stay two words: a route is what the page shows at a path, `get` what the server
answers. On the server a route path falls through to the single page, and a `get` of the same path wins over it.
`🆔int` in a route is read as `:id:int`.

## How the code splits
1. Shared code (data shapes, validation, formatting, the route table) compiles into both builds.
2. Server-only code (`server def`): the page build reads every call of a server function from a main-level variable
   `f·rpc·0` (serve.rs asking_the_server; `g := f(x)` names it g). The shipped page leaves the server functions out
   (bodies and secrets stay out of app.wasm) and fetches the value: `f·rpc·0 := fetch ["/rpc/f", [args]] ?? first
   value`, a POST of the arguments as JSON, fetched anew when an argument changes; handlers read the variable
   (`on click { n = doubled(n) }`). Probe probes/server_routes/rpc_everywhere.warp.
   - First HTML: site.rs compiles the page twice. The prerender (pipeline::prerendering) keeps the server functions,
     calls them directly, renders the HTML and exports `rpc·values`; the shipped page starts from those values
     (pipeline::with_server_values), so hydration shows the same text. headless.rs prerenders too.
   - `x := fetch url ?? default` (lowering/signals/fetch_signals.rs) is the general form: the default until the reply
     is in, and in place of a failed one. A POSTed reply is JSON of any value (src/web/fetches.rs, host-tasks.js
     fetchReply); a GET reply: objects and arrays parsed, else the text.
   - Refused, loudly: an argument the page only knows inside a function (`def show(k) { p{ doubled(k) } }`): the page
     asks before its code runs, so it sends main-level values only.
   - A server function's untyped parameter is `any`; joined with a text its result takes its text form (analyzer
     arithmetic_kind). The server build takes `g := f(x)` of a server function as `g = f(x)` (serve.rs
     assigned_asking).
3. JavaScript on the client needs no third language: browser APIs through the WebIDL bindings
   (notes/web_framework.md "web-apis: WebIDL"), arbitrary JS through foreign_call (host-foreign.js).
4. Page-only code (event handlers, DOM) is harmless on the server: it runs main once to render the first HTML, and
   handlers never fire there.
5. Server data in a route (P221: a route whose block reads server data runs on the server; nothing secret ships, the
   server gives out only what the route displays). Server data is a variable registered from the database
   (`users: [User] = database.users`, `stored users: [User]`); a plain main-level value is a build-time value.
   - serve.rs with_route_data: each value of a route's block that reads server data becomes the server function
     `route·data·N(path)`, which binds the route's parameters from the path and gives that value (ø for another
     route's path). The block calls it with the main-level `page·path`; host-routes.js navigate calls the export
     `page·navigated`, so a followed link fetches the value for the new path. The shipped page leaves the table's
     registration out. Probe probes/route_data/app.warp, tests/web/test_route_data.rs.
   - web_server.rs decodes the request path (`/rpc/route%C2%B7data%C2%B70`); a server function's text reply is
     text/plain.
   - First visit: a served page that asks the server is rendered for each request. site.rs ServedSite::file_at runs
     the prerender's module with host::with_page_path(path) and reads page·html, rpc·requests and rpc·values after
     one run of main; the page carries the replies as `<script type="application/json" id="warp-replies">`, and
     host-tasks.js answers the first fetch of each request from them before the browser paints. A Worker page
     (site-worker.js) gets them with its start message from site-thread.js.
   - Navigation asks each route-data request once (host-tasks.js routeDataReplies) until the page reloads.
     probes/route_data/server_calls.sh counts the POSTs through counting_proxy.py.
   - A main-level statement that only changes server data (`if count(users) == 0 { users.add(…) }`) runs on the
     server; the shipped page leaves it out (serve.rs changes_server_data_only, tests/web/test_page_server_data.rs).
   - Route functions stand where the first route stood (a function reads a typed main-level list from where it is
     defined). loadRouteModule loads app-route-N.wasm from the page's base.
   - Left: access rules (card route-access).
6. `warp serve [app.warp] [port]` (P222) serves any program without a `serve PORT {…}` statement: its page (site.rs),
   its `server def`s as POST /rpc/f and its top-level `get`/`post "/path" {…}` routes, at port 8080 by default (the
   file app.warp or main.warp of the folder). pipeline::serving_at(port) makes serve.rs append the serving; a program
   with its own `serve PORT {…}` keeps it. Plain `warp app.warp` serves too when serve::serves finds a route, a
   `get`/`post`, a `server def` or a `serve`, printing "serving http://localhost:8080 (…)"; `warp run` and `warp test`
   never serve. tests/web/test_warp_serve.rs.

## Tables in served routes (samples/servers/server.warp)
- Every served route (get/post/RPC) starts with `global users; users = database.users` for each table variable
  (serve.rs reading_tables), so it sees rows a page render added (tests/web/test_served_tables.rs).
- The page build leaves top-level `get`/`post` routes out (serve.rs is_serving), and the tables only they read.
- In the browser a table is kept in IndexedDB (host-files.js `table`, notes/orm.md); a filter there stays a
  comprehension, a natively compiled filter is SQL.
- ø is the empty list: a route whose value is a table, a list literal or a filter is marked (serve.rs answers_a_list,
  web_server.rs LIST_ANSWER) and answers `[]`; a `server def` giving one too (serve.rs ServerData).

## Forms (samples/storage/todo_app.warp)
A stored `Todo{title: text; done: bool}` table, a list page, a form POSTing a todo, `post "/todos/:id:int/toggle"`,
`todos where not done`; runs under `warp serve`, `warp run` and in the playground
(tests/web/test_served_forms.rs, probes/todo_app/check_served.sh).
- An urlencoded body is the map of its fields (`request.body.title`). A browser's form (Accept text/html) gets 303
  back to its Referer (post/redirect/get), any other client the route's value.
- A request's fields: `request.query.title` of a get, `request.body.title` of the other methods (the `$title`
  shorthand was dropped, decision serve-plain).
- `form post "/todos" { … }` is `form{ method:"post" action:"/todos" … }` (markup_tags::lower_form_routes, first
  source pass); any method word of serve::METHODS, any path expression.
- A bool column reads back as a bool; a one-character text is stored as its text (database.rs c_text).
- A failing route answers 500 with the failure message.
- The page build leaves the table out but keeps `id` on its class (database_tables.rs with_row_ids). A served
  instance arrives as JSON `{"Todo": {…}}` and decodes to a one-entry object read as its class (STRUCT_BODY).
- Without a server (`warp run`, the playground), top-level get/post routes are local routes (serve.rs
  with_local_routes): `route·N(request)`, with `global` for the page variables it mentions; `page·submitted(request)`
  calls the matching one, or throws "no route answers …". The playground listens for submit on the shadow root
  (playground.js submitForm), the Worker runs the route and shows the page anew (worker.js handleSubmit).
- Left: card where-assigned (`waiting = xs where not it` fails to parse).

## JSON of instances
/api and /rpc JSON shows an instance as a plain object, `{"name":"Ann","id":1}`, no class wrapper (web_server.rs
without_classes). An object's own entries (`address:{…}`) keep their names.

## Server route paths
- `get "/api/users/:id:int" { users#id }`: a server route's path takes the page routes' patterns. web_server.rs
  route_at prefers the route with exactly the path, then the first whose pattern matches (routes.rs path_matches, the
  Rust twin of lib/router.warp route_matches); serve.rs bind_path_parameters binds them from `request.path` (the
  router module is loaded for them, modules.rs ROUTE_PARTS).
- An /api/ or /rpc/ path no route takes answers JSON 404 `{"error":"no route GET …"}`, never the page.
- The single page answers every path a route matches, `route "*"` included, with 200; 404 only when no route matches
  (site.rs matches_no_route_at) or a matched route's lookup misses (`users#9`).
- Check by hand: `probes/server_edits/curl_checks.sh <warp binary> samples/servers/server.warp <port>`.

## Toward a plain library (card compiler-modules, slice serve; decision domains-plain-libraries)
Routes become ordinary warp definitions in lib/router.warp, no rewrite rules, no module pass. Moved so far:
- `status 418` / `header "name" "value"`: router functions over the host's response_header; lib/wagi.warp defines
  its own response_header, which wins over the host import in a WAGI build (serve.rs with_answer_headers is gone).
- route dispatch without a server (warp run, playground, WAGI; serve.rs with_local_routes): each route is
  `route_add(METHOD, path, request => {globals…; body})` and `page·submitted(request) := route_answer(request)`, the table
  and dispatcher in lib/router.warp; no route·N functions there any more. Served mode still calls route·N by name
  (next: the host calls one exported dispatcher; web_server.rs keeps site fallback, failure and list answers, so route
  records need path and list information). The built-in `get "/x" {…}` sugar intercepts a user function named get:
  the swap to lib functions get/post/… is one step.

Target shape (the host calls one exported dispatcher instead of route·N by name):
```warp
class Route { method: text, path: text, answer: function }
routes = []
get(path: text, answer: function) := { global routes; routes.add(Route{method: "GET", path, answer}) }
answer(request) := for route in routes { if route.method == request.method and route_matches(route.path, request.path) { return route.answer(request) } }
```
Gaps (probed 2026-10-18 with the installed warp):
- a `{…}` block to a `function` parameter is a List ("get needs a Function … got {1+2}"); question Q1 block-as-function
- a closure in a map field: `r.answer(2)` works since card field-closure-call (a field given a parameter that a call
  feeds a lambda); probes/web/plain_router.warp runs a route list and dispatcher in plain warp. Open: `a = r.answer; a(2)`
- path parameters bound into the block (`:id:int`): question Q2, card route-path-patterns (topic matching)
- decision serve-plain: `$title` dropped (done); path parameters wait for card route-path-patterns; a block to a
  function parameter is its lazy body (card block-function-argument); default routes via `functions()` (card
  functions-reflection)
Stays Rust, for card module-passes (analysis/codegen, not sugar): server functions and their /rpc routes, the page's
calls of them (asking_the_server, without_serving, prerender values), P221 route data (with_route_data), stored tables
read anew per request (reading_tables), list answers (answers_a_list), hosted page answers (answered_page) and
default routes (with_default_routes, reflection over the program's functions).
