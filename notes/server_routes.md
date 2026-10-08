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
   - Known gaps: a server function's untyped parameter is `any`, so `n = doubled(n)` makes n a data value in the native
     server build (`"n " + n` fails there; `doubled(x:int)` works). The `g := f(x)` hint "prefer g = …" still shows in
     the server build.
3. **JavaScript on the client** needs no third language in the source. The page's warp code reaches browser APIs
   through the WebIDL bindings (notes/web_framework.md "web-apis: WebIDL") and arbitrary JS through foreign_call (js =
   globalThis, host-foreign.js). Hand-written JS stays possible as a `.js` file of the site, but it is not the model.
4. **Page-only code** (event handlers, DOM, `on click`) is harmless on the server: the server runs main once to render
   the first HTML, and handlers never fire there.
5. **Server-only state** a route reads (`users` above) is a value at build/start time in the page. For live data the
   page fetches it (`users := fetch "/api/users"`). A later step could lower a route's read of a server-only variable to
   that fetch automatically. That needs a `server` mark on variables too (`server users = db.load()`), so it waits for a
   user decision.
6. **`warp serve [app.warp] [port]`** (P222, done; the CGI mode is retired): serves any program without a `serve PORT
   {…}` statement: its page (site.rs), its `server def`s as POST /rpc/f and its top-level `get`/`post "/path" {…}` routes,
   at the port (8080; the file app.warp or main.warp of the folder). pipeline::serving_at(port) makes lowering/serve.rs
   append the serving after the program's statements; a program with its own `serve PORT {…}` keeps it. Plain
   `warp app.warp` serves too when serve::serves finds a route, a `get`/`post`, a `server def` or a `serve` (a static
   check), printing "serving http://localhost:8080 (routes found; `warp run app.warp` runs it once without serving)";
   `warp run` and `warp test` never serve. Probe: probes/warp_serve/app.warp; tests/web/test_warp_serve.rs.

## Undoable defaults taken
- RPC in the page is async, like fetch (option a), starting from the prerendered value; a call with a local argument is refused.
