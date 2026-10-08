# One source for the server and its page (card route-sample, 2026-10-08)

The user's sample: `route "/" { div{…} } route "/users/:id:int" { … users#id … }`. Can the same mechanism drive a
server? How do the page's code (in the browser) and the backend logic (on the server) fit together in one source?

## Answer: mostly there already, one program and two builds
One source file is compiled twice:

| build | what it makes of | made by |
|---|---|---|
| server (native, `warp run app.warp`) | `serve 8080 { get "/api/x" {…} }` serves HTTP, `server def f` is also POST /rpc/f, the page's site answers GET / and any other path | lowering/serve.rs, web_server.rs, site::served_files |
| page (wasm in the browser, pipeline::for_a_page) | `route` patterns run in the page's router, `serve` is left out, `server def f` is asked over POST /rpc/f | lowering/routes.rs, lowering/serve.rs without_serving, site.js |

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
2. **Server-only code** (`server def`), done (card rpc-stub, probes/server_routes/rpc_stub.warp, checked in a browser):
   in the page build a main-level `x := f(args)` of a server function becomes `x := fetch ["/rpc/f", [args]]`
   (lowering/serve.rs without_serving), a POST of the arguments as a JSON array. It is async like any fetch (option a):
   `x` is ø with `x.loading` until the reply arrives, so the page writes `x ?? "…"` (the ø check demands it). f's
   definition is left out of the page, so its body and any secret in it stay out of app.wasm
   (tests/web/test_server_rpc_stub.rs). fetch_start takes `[url, body]` for a POST on both hosts (src/host.rs
   request_of, host-tasks.js startFetch).
   - A page that calls f any other way (`p{ f() }`, inside a function) still gets f's body, with a warning naming f and
     the `x := f(…)` form. A synchronous call (option b) would need site-worker's Atomics.wait and coi-serviceworker.
   - The page's first HTML (prerendered at build time by app.wasm itself) shows the pending state, `said: …`.
3. **JavaScript on the client** needs no third language in the source. The page's warp code reaches browser APIs
   through the WebIDL bindings (notes/web_framework.md "web-apis: WebIDL") and arbitrary JS through foreign_call (js =
   globalThis, host-foreign.js). Hand-written JS stays possible as a `.js` file of the site, but it is not the model.
4. **Page-only code** (event handlers, DOM, `on click`) is harmless on the server: the server runs main once to render
   the first HTML, and handlers never fire there.
5. **Server-only state** a route reads (`users` above) is a value at build/start time in the page. For live data the
   page fetches it (`users := fetch "/api/users"`). A later step could lower a route's read of a server-only variable to
   that fetch automatically. That needs a `server` mark on variables too (`server users = db.load()`), so it waits for a
   user decision.
6. **`warp serve app.warp [port]`**: today `warp run` serves when the program says `serve PORT {…}`. A CLI word that
   serves any page program (site + /rpc routes, no `serve` statement needed) would be the production twin of
   `warp dev`. Cheap: main.rs plus a default port. Not done, because the old `serv` CGI mode in main.rs holds the word.
   Retiring that mode is a question for the user.

## Undoable defaults taken
- RPC in the page is async, like fetch (option a); a direct call in the page ships the body, with a warning.
