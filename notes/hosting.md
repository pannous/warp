# Hosting warp programs (card cloud-hosting, cloud-deploy step 2)

The user asked on 2026-10-09: "determine the hosting options; let's start with our own login and let people log in with
their own button". Step 1, `warp deploy` through wrangler, belongs to warp-web. This note covers hosting and login.

## What a host has to do
A warp program is one WASM GC module (`warp compile --wasm`). A pure one has no imports, and printing imports only
`wasi_snapshot_preview1.fd_write`. A host therefore needs (1) WASM GC, (2) a way to load *this* module, and (3)
glue that turns a request into a call and the returned Node into a response. The glue is warp-web's Worker glue.

## Verified by running a warp module (probes/hosting/)
| host | engine | GC | how checked |
|---|---|---|---|
| Cloudflare Workers | V8 (workerd) | **yes** | gc_worker/ under `wrangler dev`, and deployed to warp-gc-check.pannous.workers.dev (since deleted): `{"gc":"object","kind":"262"}` |
| Cloudflare Workers, wasm compiled at run time | | **refused** | `WebAssembly.compile(bytes)` → "Wasm code generation disallowed by embedder". Each program must be uploaded as its own script, with the .wasm as a module part |
| Spin 4.2.2 (Akamai Functions, wasmtime) | wasmtime | **yes** | spin_check/ WAGI: answers `fib 6765`. Spin componentizes the module, and `_start` has to be `() -> ()`, so warp's `main` (which returns a Node) needs a void wrapper. Spin 3.0 failed. WAGI is deprecated, and the long-term route is a wasi-http component (`warp build --component`) |
| Deno 2 (Deno Deploy, Netlify Edge) | V8 | **yes** | `deno run --allow-read deno_check.mjs` → `object 262` |

## Ranked options
1. **Cloudflare Workers (recommended, built).** GC is verified on the edge. The free plan allows 100 scripts per
   account and 100k requests/day, and Workers Paid is $5/month. Workers for Platforms costs $25/month (1,000
   scripts, 20M requests, dispatch namespaces with no script limit, `<name>.warp.pannous.com` through one dispatch
   Worker) and isn't bought yet (API error 10121). Login for **our** hosting: GitHub OAuth. Login for **their own
   account**: Cloudflare OAuth clients have been open to third parties since 2026 (authorization code + PKCE; scopes
   `workers-scripts.write`, `user-details.read`, `account-settings.read`). A client is private (members of our
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
- Also possible: **our own server** (pannous.com) running `warp serve` natively (wasmtime, SQLite, files) behind a
  wildcard nginx vhost. It's the only option with the full native host (database, files), at the cost of
  operating it ourselves.

## Built: warp-hosting (web/hosting/)
One control Worker in our Cloudflare account, https://warp-hosting.pannous.workers.dev:
- `GET /auth/github` → GitHub OAuth → a signed session cookie (HMAC, `SESSION_SECRET`). A CLI or test sends
  `Authorization: Bearer <GitHub token>` instead (`gh auth token`), checked against api.github.com/user.
- `POST /deploy?name=<name>` with the program's .wasm as the body: the name is owned by the first GitHub user who
  deploys it (KV `NAMES`). The Worker builds a script from its glue plus the wasm and uploads it as
  `warp-<name>` (Cloudflare API, `CLOUDFLARE_API_TOKEN`), with its workers.dev route enabled. It answers
  `{url: "https://warp-<name>.pannous.workers.dev"}`.
- "Deploy to my own account": the same upload with the user's own token and account (`X-Cloudflare-Token`,
  `X-Cloudflare-Account`), which the browser keeps and the Worker passes through without storing. The
  api.cloudflare.com API has no CORS, which is why this goes through our Worker.
- The playground's ⋯ menu has Deploy and "Deploy to my Cloudflare".

## Open (user decisions, sent to the Interviewer 2026-10-09)
GitHub OAuth app + secrets, the Cloudflare API token for uploads, Workers for Platforms ($25/month), pannous.com
nameservers (mixed orderbox + Cloudflare, so a `*.warp.pannous.com` wildcard is unreliable), and a Cloudflare OAuth
client (private now, public after the TXT record).
