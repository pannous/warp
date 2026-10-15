# Hosting warp programs (card cloud-hosting, cloud-deploy step 2)

Roadmap card: hosting (board topic; its sub-cards are the steps and leftovers below, notes/roadmaps.md).

The user asked on 2026-10-09: "determine the hosting options; let's start with our own login and let people log in with
their own button". Step 1, `warp deploy` through wrangler, belongs to warp-web. This note covers hosting and login.

## What a host has to do
A warp program is one WASM GC module (`warp compile --wasm`). A pure one has no imports, and printing imports only
`wasi_snapshot_preview1.fd_write`. A host therefore needs (1) WASM GC, (2) a way to load *this* module, and (3)
glue that turns a request into a call and the returned Node into a response. The glue is warp-web's Worker glue.

## Verified by running a warp module (probes/hosting/gc_hosts.sh, `--edge` deploys to workers.dev too)
| host | engine | GC | how checked |
|---|---|---|---|
| Cloudflare Workers | V8 (workerd) | **yes** | `wrangler dev`, and deployed to warp-gc-check.pannous.workers.dev (since deleted): `{"gc":"object","kind":"262"}` |
| Cloudflare Workers, wasm compiled at run time | | **refused** | `WebAssembly.compile(bytes)` → "Wasm code generation disallowed by embedder". Each program must be uploaded as its own script, with the .wasm as a module part |
| Spin 4.2.2 (Akamai Functions, wasmtime) | wasmtime | **yes** | WAGI: answers `fib 6765`. Spin componentizes the module, and `_start` has to be `() -> ()`, so warp's `main` (which returns a Node) needs a void wrapper. Spin 3.0 failed. WAGI is deprecated, and the long-term route is a wasi-http component (`warp build --component`) |
| Deno 2 (Deno Deploy, Netlify Edge) | V8 | **yes** | `deno run` → `{"gc":"object","kind":"262"}` |

## Ranked options
1. **Cloudflare Workers (recommended, built).** GC is verified on the edge. The free plan allows 100 scripts per
   account and 100k requests/day, and Workers Paid is $5/month. Workers for Platforms costs $25/month (1,000
   scripts, 20M requests, dispatch namespaces with no script limit, `<name>.warp.pannous.com` through one dispatch
   Worker) and isn't bought yet (API error 10121). Login for **our** hosting: GitHub OAuth. Login for **their own
   account**: Cloudflare OAuth clients have been open to third parties since 2026 (authorization code + PKCE; scopes
   `workers-scripts.edit`, `user-details.read`, `account-settings.read`; `offline_access` is refused for our client). A client is private (members of our
   account only) until a TXT record `cloudflare_oauth_client_publisher=…` on the client's domain makes it public.
   The fallback needs no registration: a scoped API token the user pastes.
2. **Netlify Edge Functions (Deno).** GC works because Deno's V8 does, and Edge Functions compile wasm from bytes
   (Netlify's own wasm example uses `new WebAssembly.Module(bytes)`), so one function could serve every program from
   storage. It also has the most mature third-party OAuth ("public integration … must use OAuth2", deploy API with
   file digests). That makes it the best second "deploy to my own account" target.
3. **Deno Deploy.** V8, GC verified locally. Free: 1M requests/month, about 10 CPU-hours (cut in Aug 2026), Pro $20.
   It has no third-party OAuth for deploying, only access tokens.
4. **Spin / Akamai Functions (Fermyon).** wasmtime, GC verified locally with Spin 4.2. Akamai bought Fermyon in
   Dec 2025. The old Fermyon Cloud free tier (5 apps, 100k requests) isn't confirmed under Akamai. It has no
   third-party OAuth. Worth it once warp builds wasi-http components.
5. **Fastly Compute.** Runs Fastly's own wasmtime build. Wasmtime ≥ 47 enables GC by default, but Fastly doesn't
   document it, so this is unverified (it needs an account). Needs the Fastly ABI or a wasi-http component. API tokens
   only, no OAuth for third parties.
6. **Vercel.** Edge runtime deprecated (Next.js 16.3 drops `runtime = 'edge'`); Node functions run GC (V8). Sign in
   with Vercel is identity only; deploying needs an Integration's OAuth with the `deployment` scope.
7. **wasmCloud.** Self-hosted wasmtime with components. No hosted free tier to compare.
8. **AWS Lambda.** No native wasm: a Node 22 handler instantiates the module (V8, GC fine). IAM keys only, so it's
   the heaviest for users to set up.
- **Our own server** (pannous.com, built: "Built: warp-lambda" below): `warp --sandbox serve` natively (wasmtime,
  SQLite, files) behind Ferron. The only option with the full native host, at the cost of operating it ourselves.

## Built: warp-hosting (web/hosting/)
One control Worker in our Cloudflare account, https://warp-hosting.pannous.workers.dev, reached as
https://lambda.pannous.com (a Ferron reverse proxy on pannous.com, ~/dev/pannous-lockdown notes/rust-web.md, until
lambda's DNS moves to Cloudflare):
- `GET /auth/github?origin=<playground>` → GitHub OAuth app (callback https://lambda.pannous.com/callback) → the
  callback page posts `{warpHosting: {session, login}}` to the playground that opened it (a signed `warp.…` session,
  HMAC `SESSION_SECRET`, 30 days). Opened directly, the page says "Logged in as <login>". The CLI and the test send
  `Authorization: Bearer <GitHub token>` instead (`gh auth token`), checked against api.github.com/user.
- `GET /auth/cloudflare?origin=…` → Cloudflare OAuth client "warp hosting" (PKCE, callback …/callback/cloudflare) →
  `{warpHosting: {cloudflareToken}}`. The client is public and verified since 2026-10-15 (it was private until the publisher TXT record
  on lambda.pannous.com); the API-token paste (Workers Scripts: Edit) stays as fallback.
  Scopes (card cf-deploy-access): `workers-scripts.edit` is the role "Workers Editor", which updates existing Workers
  but cannot create one, so a new program's upload failed "No access to the specified resource". The login asks for
  `workers-scripts.admin` (Workers Admin) + `user-details.read` + `account-settings.read`. Each must be registered on
  the client, else the authorize page refuses the whole login (invalid_scope). Which scopes the client has can be read
  without a login: `/oauth2/auth?client_id=…&state=x&scope=<one>` redirects with error=invalid_scope for an
  unregistered one (probes/hosting/oauth_scopes.sh over `GET /client/v4/oauth/scopes`). On 2026-10-15 it had:
  account-settings.read user-details.read workers-scripts.edit workers-routes.write workers-ci.write containers.write
  workers-kv-storage.write workers-observability(-telemetry).write workers-r2(-bucket-item).write r2-catalog.write.
  Errors name the call (`Cloudflare PUT workers/scripts/x: …`); Cloudflare's 9109 "Invalid access token" is a 401
  (the playground drops the login and asks for a click), any other 403 is a missing permission and stays a 403. An
  account without a workers.dev subdomain gets a 409 pointing to the dashboard.
- `POST /deploy?name=<name>&scripts=<host scripts>` with the module as the body: the first GitHub user to deploy a
  name owns it (KV `NAMES`). worker.js = the named playground scripts + cloud-worker.js (warp-web's glue), uploaded
  as `warp-<name>` with app.wasm and app.bin, workers.dev route on: `{url: "https://warp-<name>.pannous.workers.dev"}`.
  `DELETE /deploy?name=` removes one's own program.
- "Deploy to my own account": the same upload with `X-Cloudflare-Token` (and `X-Cloudflare-Account` when the token
  reaches several accounts), passed through, never stored. api.cloudflare.com has no CORS, hence the Worker.
- Login tickets (card deploy-opens-tab): GitHub's login page cuts the popup off from the playground (COOP: `opener`
  is null afterwards, so postMessage never arrived and the user saw "Logged in as …, you can close this window" with
  nothing deployed). The playground sends `&ticket=<random>` to /auth/…; the callback stores the login in KV under
  `ticket:<ticket>` (10 minutes) and the playground polls `GET /ticket?ticket=`. After deploying it posts
  `POST /ticket?ticket=&program=<url>` (only *.workers.dev and *.lambda.pannous.com addresses) or `&error=`, and the
  login window, waiting on the same ticket, turns into the program's tab. Already logged in, the click opens a tab
  at once at the same deploy window: only a click may open a window, so each deploy opens exactly one.
- The deploy window (card user-click; user: "they should give one the link or a redirect to the link"):
  `GET /deploying?ticket=` shows the program's link as soon as the ticket holds it (`/deploy?…&ticket=` stores the
  address once uploaded, an error replaces it and stays shown with its reason; the playground posts it again, and
  native deploys only post it), and opens the program
  once `GET /up?program=` says it answers: Cloudflare's "error code: 1042" means no Worker under that name yet, a new
  *.lambda.pannous.com name fails until its certificate is issued. After a minute without an answer the window says
  so and keeps the link. /up fetches Workers of our own account, which Cloudflare refuses (1042) unless the control
  Worker has the compatibility flag `global_fetch_strictly_public`. Probe: probes/hosting/login_ticket.sh runs the
  window's script in node (deploying_window.mjs).
- Names of the account's own Workers (warp-hosting itself, warp-assistant) can't be claimed: `claim` refuses a name
  whose warp-<name> Worker exists without an owner in NAMES (before 2026-10-12 a GitHub user could have overwritten
  the control Worker by deploying the name `hosting`).
  An expired session asks for another click. `claim` leaves ticket keys out of its program counts.
- Clients: `warp deploy --hosted app.warp` (src/web/deploy.rs; WARP_HOSTING, WARP_HOSTING_TOKEN); the playground's ⋯ menu
  (deploy.js: Deploy, Deploy to my Cloudflare, a pasted token; the browser compiler builds the module and names the
  scripts, src/web/mod.rs worker_bundle → src/host/host_parts.rs).
- Tests: tests/web/test_deploy.rs (the bundle); `node web/hosting/test_hosting.mjs` against the real API (wrangler
  dev of the control Worker, `warp deploy --hosted`, the live routes, DELETE; needs ~/.keys CLOUDFLARE_API_TOKEN and gh).

## Browser programs (card deploy-browser-site)
A program that uses the browser (paint, use draw, the GPU, sound, text on a canvas, the clipboard, `on frame` and
other page events: host_parts.rs uses_the_browser) has nothing to answer on a server: the Worker would fail at its
first request ("host.signal_listen is not available"). Deploy makes it a site instead (host_parts.rs `deployable` →
`Deployable::Site`): the module compiled for a page, the page's scripts, those of the program's Worker and the files
shipped (src/host/page_parts.rs, the layout `warp build --site` uses, which builds on wasm32 too, so the playground's
Deploy computes it). The Worker is `const SITE = {title, scripts, workerScripts, files}` + web/playground/cloud-site.js:
it serves the page (empty markup, the program shows itself once it runs; no native prerender), the scripts and
app.wasm (app.bin's bytes), all with COOP/COEP so the page and the program's Worker share memory. warp-hosting gets
`?site=…&worker=…&files=…` instead of `?scripts=…` and composes the same Worker from its own copies (SITE_FILES).
Page routes (`route "/x" {…}`) work in the page. Server parts (a top-level `get "/api" {…}`, `serve PORT {…}`, a
`server def`; serve.rs has_server_parts) have no server in a site: Deploy refuses such a program, naming card
program-server (CLI and playground alike), rather than dropping them silently. Serving both from one Worker is open:
`warp serve` runs main on the server too (paint writes a png there), while a Worker's host has no browser words, so
the server's module would need main without them, or the Worker host a decided meaning for them.

## Built: warp-lambda, native on pannous.com (card ferron-hosting)
- Playground ⋯ menu "☁ Deploy to pannous.com" (same GitHub login as ☁ Deploy) sends the program's source to
  https://lambda.pannous.com/native/deploy?name=x; it answers at https://x.lambda.pannous.com. Demo:
  https://lambda-demo.lambda.pannous.com/fib/20.
- web/hosting/server/: warp_lambda.py (Python stdlib daemon on 127.0.0.1:8890, user warp-lambda; who deploys comes
  from the Worker's /me; names.json keeps owners; 20 programs, 5 per user), warp-lambda@.service (one unit per
  program), warp-lambda.service, warp-lambda.rules (polkit: the daemon may start/restart/stop/clean only
  warp-lambda@*.service), install.sh (over ssh, optional Linux binary:
  `cargo zigbuild --release --target x86_64-unknown-linux-gnu.2.39 --bin warp`).
- Sandboxed (user, 2026-10-09): `warp --sandbox` grants Host, Wasi, Libm, Sql only (no C/FFI, shell, foreign
  runtimes: "capability denied" at deploy). The rule at run time (src/host/sandbox.rs, cards sandbox-exec and
  sandbox-hosted, 2026-10-13): no process (os.exec behind `exec sh"…"`, which std_io hid from the capability check;
  sound players, the MIDI sender), loud "capability denied … warp --sandbox"; files are written only inside the state
  folder (systemd's STATE_DIRECTORY, else the working directory): file.write/write_bytes/append, stored values,
  SQLite files, midi.write_file, sound.render, the path resolved, `..` and symbolic links refused; fetch and net reach
  the network (the unit's IPAddressDeny keeps it off the server itself). The unit: DynamicUser, own state folder
  /var/lib/warp-lambda-data/<name> (wiped by `systemctl clean` on remove), the source read-only, ProtectSystem=strict,
  192 MB, half a core, 64 tasks. Network: the program listens on 127.0.0.2; IPAddressDeny of localhost, private
  ranges and the server's own IP with only 127.0.0.2 allowed, so it reaches the internet but not the ssh gateway
  (127.0.0.1:8888) or the daemon; the daemon connects from 127.0.0.2 (source_address), any other source is dropped.
- Ferron (pannous-lockdown ferron.kdl): `*.lambda.pannous.com` proxies to the daemon (Host kept) with on-demand
  Let's Encrypt certificates, `auto_tls_on_demand_ask` → /native/ask (200 only for deployed names), cached in
  /var/lib/ferron/acme. The first HTTPS request to a new name fails while it's issued, so deploy fetches the URL
  until it works (~15 s) and says `certificate: pending` otherwise. Let's Encrypt: 50 new certificates per week
  for pannous.com.
- Test: `node web/hosting/test_lambda.mjs` (daemon with child processes instead of systemd, the real /me, and the
  playground button via agent-browser).

## Limits (no surprise bills)
- Our account is on the Workers Free plan, which never bills: past 100k requests/day the Workers answer errors until
  the next day, and the account holds 100 scripts (6 are the user's own).
- warp-hosting caps programs: 80 in total (MAX_PROGRAMS, "full for now: deploy to your own account") and 5 per GitHub
  user (MAX_PROGRAMS_PER_USER), wrangler.toml vars.
- On Workers Paid ($5/month) usage would bill without a hard cap: then add billing notifications and a rate limit on
  /deploy before switching.

## Decisions (user, via the Interviewer, 2026-10-09)
Free plan now, programs at warp-<name>.pannous.workers.dev, no Workers for Platforms yet; GitHub login with the
callback on lambda.pannous.com behind a proxy on pannous.com (not workers.dev); a private Cloudflare OAuth client now
with the token-paste fallback. Open: publishing the Cloudflare client (TXT record; card oauth-client-publish), pannous.com nameservers (card warp-subdomains).
State on 2026-10-12 (sent to the Interviewer as user steps):
- The registrar (alpinedomains / orderbox) delegates to the 4 orderbox nameservers only, but the orderbox zone also
  lists houston/sky.ns.cloudflare.com. Cloudflare's pannous.com zone is "pending" and answers authoritatively with
  empty answers (`dig A pannous.com @sky.ns.cloudflare.com`: NOERROR, aa, 0 answers), so a resolver that asks it
  caches "no address" for pannous.com, lambda.pannous.com and the mail records. Fix: delete those two NS records in
  the orderbox zone. Moving the domain to Cloudflare instead would change pannous.com itself: that zone points it at
  Workers (AAAA 100::) and has no lambda record.
- Publishing the OAuth client needs the TXT record on the client URL's host. warp.pannous.com is a CNAME to GitHub
  Pages, so no TXT can go there: the client URL should be https://lambda.pannous.com (A record) with the TXT on
  `lambda`, then visibility Public in the dashboard. Our API token can't read or edit OAuth clients.
- 2026-10-14: adding that TXT (zone serial 2026101301) left `lambda` and `*.lambda` without A records at orderbox:
  lambda.pannous.com stopped resolving, while the server still answers on 81.169.181.160. Both A records have to be
  added back next to the TXT (one name can have A and TXT together).

## Decisions (moved from notes/decisions.md, 2026-10-09)
- Each program is its own Cloudflare Worker uploaded by the hosting Worker. Q1 GitHub OAuth App for "Log in with
  GitHub" (callback https://lambda.pannous.com/callback, proxied by pannous.com to warp-hosting.pannous.workers.dev);
  Q2 a Cloudflare "Edit Cloudflare Workers" API token for uploads; Q3 free Workers plan (~90 programs, 100k
  requests/day), Workers for Platforms only near the cap; Q4 programs at warp-<name>.pannous.workers.dev,
  <name>.warp.pannous.com later; Q5 a private Cloudflare OAuth client for one's own account, others paste a scoped
  token kept in their browser.
- Fermyon / Akamai Functions: a live test of "☁ Deploy to Fermyon" needs the user's personal access token in .env as
  akamai_functions_token (card fermyon-token); visitors paste their own token, kept in their browser.
