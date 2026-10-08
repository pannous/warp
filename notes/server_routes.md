# One source for the server and its page (card route-sample, 2026-10-08)

The user's sample: `route "/" { div{…} } route "/users/:id:int" { … users#id … }`. Can the same mechanism drive a
server? How do the page's code (in the browser) and the backend logic (on the server) fit together in one source?

## Answer: mostly there already, one program and two builds
One source file is compiled twice:

| build | what it makes of | made by |
|---|---|---|
| server (native, `warp run app.warp`) | `serve 8080 { get "/api/x" {…} }` serves HTTP, `server def f` is also POST /rpc/f, the page's site answers GET / and any other path | lowering/serve.rs, web_server.rs, site::served_files |
| page (wasm in the browser, pipeline::for_a_page) | `route` patterns run in the page's router, `serve` is left out, each call of a `server def f` is a value fetched from POST /rpc/f | lowering/routes.rs, lowering/serve.rs without_serving, site.js |

Example (probes/server_routes/one_source.warp, which runs and was checked in a browser):

```warp
users = ["Ann", "Bo"]
server def user_count() { count(users) }
serve 18461 { get "/api/users" { users } }
route "/" { div{ h1{ "Users" } a{ href:"/users/1" "the second" } } }
route "/users/:id:int" { p{ "User " + users#id } }
route "*" { p{ "no such page" } }
```

- GET /api/users gives `["Ann","Bo"]`, and POST /rpc/user_count gives `2`.
- GET /users/1 is the one page (index.html, `<base href="/">`), and its router shows "User Ann" once it has hydrated.
  A link from / navigates without a reload.
- `route` (pages) and `get` (data) stay two words. A route is what the page shows at a path, and `get` is what the
  server answers. On the server a route path falls through to the page (single-page, user decision 2026-10-07: "I much
  preferred the single HTML"), and a `get` of the same path wins over it.
- The sample's `🆔int` is read as `:id:int` (route-typed). An emoji parameter mark would be an alias question for the
  Interviewer only if the user meant it literally.

## Fixed now (commit 366c0efdc)
A program with `server def` and a page failed its page build ("undefined: server"), so the server served no page at
all. The page build now takes the definition: tests/web/test_server_functions_in_page.rs.

## What is left: how the code splits (the real design question)
1. **Shared code** (data shapes, validation, formatting, the route table) compiles into both builds. It works today
   because every definition is in both.
2. **Server-only code** (`server def`), done (cards rpc-stub, rpc-everywhere; probes/server_routes/rpc_everywhere.warp,
   checked in a browser: the button's handler doubles n through the server, "next" is fetched anew):
   - The page build reads **every** call of a server function from a main-level variable, `f·rpc·0` (lowering/serve.rs
     asking_the_server; `g := f(x)` names it g). The shipped page leaves the server functions out (their bodies and
     secrets stay out of app.wasm) and fetches the value: `f·rpc·0 := fetch ["/rpc/f", [args]] ?? first value`, a
     POST of the arguments as JSON, fetched anew when an argument changes. Markup, expressions and handlers read the
     variable (`on click { n = doubled(n) }` takes the value for the current n).
   - **First HTML**: site.rs compiles the page twice. The prerender (pipeline::prerendering) keeps the server
     functions, calls them directly as the server does, renders the HTML and exports `rpc·values`; the shipped page
     starts from those values (pipeline::with_server_values), so hydration shows the same text and the ø check is
     satisfied. headless.rs prerenders too.
   - `x := fetch url ?? default` (lowering/fetch_signals.rs) is the general form: the default until the reply is in,
     and in place of a failed one. A POSTed reply is JSON of any value, a number or a text too (src/fetches.rs,
     host-tasks.js fetchReply); a GET reply stays as before (objects and arrays parsed, else the text).
   - Refused, loudly: an argument the page only knows inside a function (`def show(k) { p{ doubled(k) } }`). The page
     asks before its code runs, so it sends main-level values only. A synchronous call would need site-worker's
     Atomics.wait.
   - A server function's untyped parameter is `any`, so its arithmetic is a number of run-time kind; joined with a
     text it takes its text form (`"n " + doubled(n)`, analyzer arithmetic_kind). The server build takes `g := f(x)` of
     a server function as `g = f(x)`, the value it renders, as the prerender does (serve.rs assigned_asking), so the
     := hint no longer shows there (card server-def-any).
3. **JavaScript on the client** needs no third language in the source. The page's warp code reaches browser APIs
   through the WebIDL bindings (notes/web_framework.md "web-apis: WebIDL") and arbitrary JS through foreign_call (js =
   globalThis, host-foreign.js). Hand-written JS stays possible as a `.js` file of the site, but it is not the model.
4. **Page-only code** (event handlers, DOM, `on click`) is harmless on the server: the server runs main once to render
   the first HTML, and handlers never fire there.
5. **Server data in a route** (P221, user decision: a route whose block reads server data runs on the server, nothing
   secret ships, the server gives out only what the route displays). Server data is a variable registered from the
   database (`users: [User] = database.users`, `stored users: [User]`, `prefs = database.prefs`). A plain main-level
   value (`users = ["Ann", "Bo"]` above) is still a value at build time in the page.
   - lowering/serve.rs with_route_data: each value of a route's block that reads server data (`"User " + users#id.name`
     inside `h1{…}`; markup and blocks are looked into, any other value is asked for whole) becomes the server function
     `route·data·N(path)`. It binds the route's parameters from the path and gives that value, ø for a path of another
     route. So POST /rpc/route·data·N with `["/users/2"]` gives "User Bo" and nothing else of the table.
   - The block calls it with the main-level `page·path` (page_path()), so the page asks through rpc-everywhere (item 2).
     host-routes.js navigate calls the export `page·navigated`, which sets page·path anew, so a followed link fetches
     the value for the new path. The shipped page leaves the table's registration out (app.wasm has no table name).
   - web_server.rs decodes the request path (`/rpc/route%C2%B7data%C2%B70`). A server function's text reply is text/plain
     and arrives as that text (host-tasks.js fetchReply; before, it got the `\n` of a GET reply).
   - Probe: probes/route_data/app.warp (seed command in its comment), checked in a browser: / → first → second → home,
     and a direct visit to /users/2. Tests: tests/web/test_route_data.rs.
   - First visit (step 3): a served page that asks the server (its prerender exports rpc·values) is rendered for each
     request. site.rs ServedSite::file_at runs the prerender's module with host::with_page_path(path) and reads
     page·html, rpc·requests (`[["/rpc/f", [args]] …]`) and rpc·values after one run of main. The page carries those
     replies as `<script type="application/json" id="warp-replies">`. host-tasks.js answers the first fetch of each
     request from them in a microtask, so the page shows the rendered value before the browser paints. A page without
     server calls stays the built file. Test: the_first_visit_gets_finished_html.
   - Navigation asks each route-data request once (card route-data-cache): host-tasks.js keeps the replies of
     `/rpc/route·data·N` in by request (routeDataReplies), so going back to a page, or to a path of another route (ø),
     asks the server nothing; the reply stays as fetched until the page reloads.
   - A page whose program runs in a Worker (site-worker.js) gets the replies element's text with its start message
     from site-thread.js, so its first visit asks the server nothing either (card ssr-worker-replies).
   - Probe of both: probes/route_data/server_calls.sh counts the POSTs through probes/route_data/counting_proxy.py (the
     browser's network log misses a Worker's fetches): app.warp's links home, first, second, home, first, second ask
     "/users/1" and "/users/2" once each; worker_app.warp's direct visit of /users/2 asks nothing (one call before).
   - Left: (c) A main-level statement of the page that reads server data (`if count(users) == 0
     { users.add(…) }`) fails loudly in the browser ("table.open: no such word in the browser"). (d) Access rules: card
     route-access.
   - Found on the way: route functions now stand where the first route stood instead of before everything, because a
     function reads a typed main-level list from where it is defined (cards typed-global-capture, route-typed-list).
     loadRouteModule loads app-route-N.wasm from the page's base as loaded (a link followed from / asked
     /users/app-route-1.wasm).
6. **`warp serve [app.warp] [port]`** (P222, done; the CGI mode is retired): serves any program without a `serve PORT
   {…}` statement: its page (site.rs), its `server def`s as POST /rpc/f and its top-level `get`/`post "/path" {…}` routes,
   at the port (8080; the file app.warp or main.warp of the folder). pipeline::serving_at(port) makes lowering/serve.rs
   append the serving after the program's statements; a program with its own `serve PORT {…}` keeps it. Plain
   `warp app.warp` serves too when serve::serves finds a route, a `get`/`post`, a `server def` or a `serve` (a static
   check), printing "serving http://localhost:8080 (routes found; `warp run app.warp` runs it once without serving)";
   `warp run` and `warp test` never serve. Probe: probes/warp_serve/app.warp; tests/web/test_warp_serve.rs.

## Undoable defaults taken
- RPC in the page is async, like fetch (option a), starting from the prerendered value; a call with a local argument is refused.
