# One source for the server and its page (card route-sample, 2026-10-08)

The user's sample: `route "/" { div{…} } route "/users/:id:int" { … users#id … }`. Can the same mechanism drive a
server? How do the page's code (in the browser) and the backend logic (on the server) fit together in one source?

## Answer: mostly there already, one program and two builds
One source file is compiled twice:

| build | what it makes of | made by |
|---|---|---|
| server (native, `warp run app.warp`) | `serve 8080 { get "/api/x" {…} }` serves HTTP, `server def f` is also POST /rpc/f, the page's site answers GET / and any other path | lowering/serve.rs, web_server.rs, site::served_files |
| page (wasm in the browser, pipeline::for_a_page) | `route` patterns run in the page's router, `serve` is left out, `server def f` is a plain function | lowering/routes.rs, lowering/serve.rs without_serving, site.js |

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
all. The page build now keeps the definition as a function of the page: tests/web/test_server_functions_in_page.rs.

## What is left: how the code splits (the real design question)
1. **Shared code** (data shapes, validation, formatting, the route table) compiles into both builds. It works today
   because every definition is in both.
2. **Server-only code** (`server def`): its body still runs in the page too. This works for pure code, but it is wrong
   for two kinds of code:
   - code that reads server state (a database, files, secrets);
   - secrets shipped in app.wasm, where anyone can read them. **This is a leak once real backends exist.**

   Step 2 (next card, rpc-stub): the page build replaces the body with a call of POST /rpc/f. It sends the arguments as
   a JSON array and reads the result as the value. Natively the call is direct, so the server's own render needs no
   HTTP. Two ways to make the page wait:
   - (a) Like `users := fetch …`: a main-level `n := user_count()` becomes a fetch signal (loading/value/error). This is
     the existing async model, so the same lowering as fetch_signals.rs with a POST. **Default.**
   - (b) A synchronous call. That only works inside site-worker (Atomics.wait), so every page with RPC would need the
     Worker plus coi-serviceworker, which costs the bundle budget.
3. **Page-only code** (event handlers, DOM, `on click`) is harmless on the server: the server runs main once to render
   the first HTML, and handlers never fire there.
4. **Server-only state** a route reads (`users` above) is a value at build/start time in the page. For live data the
   page fetches it (`users := fetch "/api/users"`). A later step could lower a route's read of a server-only variable to
   that fetch automatically. That needs a `server` mark on variables too (`server users = db.load()`), so it waits for a
   user decision.
5. **`warp serve app.warp [port]`**: today `warp run` serves when the program says `serve PORT {…}`. A CLI word that
   serves any page program (site + /rpc routes, no `serve` statement needed) would be the production twin of
   `warp dev`. Cheap: main.rs plus a default port. Not done, because the old `serv` CGI mode in main.rs holds the word.
   Retiring that mode is a question for the user.

## Undoable defaults taken
- `server def` bodies run in the page until rpc-stub (loud in this note, not silently). The page build used to fail on them.
- RPC in the page will be async, like fetch (option a).
