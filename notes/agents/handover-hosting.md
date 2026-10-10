# Handover: warp-hosting (2026-10-10)

Retired with no hosting cards left; the next warp-hosting starts here.

## Assistant proxy (card assistant-proxy, done)
The playground assistant for visitors without their own Anthropic key.
- Worker `warp-assistant` at https://warp-assistant.pannous.workers.dev, from web/assistant/assistant.mjs +
  wrangler.toml (deploy: `wrangler deploy` in web/assistant). It answers only Origin https://warp.pannous.com
  (PLAYGROUND_ORIGIN), after a Turnstile check: POST /session gives an HMAC session for an hour, POST /messages pins
  `proxy_model`, each task's max_tokens and the system prompt from web/playground/assistant.json. Rate limit
  PER_ADDRESS 20 per 60 s.
- Client: web/playground/assistant.js `askClaude` goes direct with a visitor's key, else `askProxy` + Turnstile
  (public site key in TURNSTILE_SITE_KEY there).
- Secrets live in ~/.keys (ASSISTANT_ANTHROPIC_KEY of Console workspace warp-playground with a monthly spend limit,
  TURNSTYLE_SECRET; TURNSTYLE_KEY is the public site key). `~/dev/bin/warp-assistant-secrets` copies them into the
  Worker by stdin and rotates SESSION_SECRET (ends open sessions). Never print, commit or relay them. Cloudflare
  credentials: CLOUDFLARE_OAUTH_CLIENT_ID / _SECRET in /Users/me/dev/angles/warp/.env.
- Test: tests/web/test_assistant_proxy.rs runs web/assistant/test_assistant.mjs against the live Worker (Origin 403,
  forged sessions and dummy Turnstile tokens refused, the workspace key answers). A keyless end-to-end run needs a
  browser, so the user's first try is the first full run.

### After the user's first keyless try
- `wrangler tail warp-assistant` while they try: /session must give 200 (a 403 there is Turnstile: check the widget's
  hostnames in the Cloudflare Turnstile dashboard), /messages 200, no 429 from normal typing (else raise PER_ADDRESS).
- Console workspace warp-playground: usage appears and stays under the spend limit.
- If completion feels slow or terse, the knobs are in assistant.json (model, per-task tokens), shared with the
  direct-key path.

## Open, waiting on the user (board cards)
- user-click / cloud-deploy: the user clicks ☁ Deploy to my Cloudflare in the playground and reports the status line.
- user-sign / fermyon-hosting: an Akamai Functions token in .env as akamai_functions_token, then test live. The
  token is never printed or committed.

## Rules that held
Hosted programs must not reach localhost services. No local browser: browser checks via the Playground workflow on
a branch. Deploy is part of done: a playground change is finished when the Pages run is green and the file is live
(curl it).
