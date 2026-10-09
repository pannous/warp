# agent "prompt" and the playground's assistant (cards g_X_F0, g_X_Uw, 2026-10-09)

- `agent "prompt"` / `agent(prompt, model)`: lib/agent.warp, loaded when a program calls agent (IMPLICIT_MODULES in
  src/modules.rs, like units), so hello world does not grow. One POST to https://api.anthropic.com/v1/messages, the
  answer's first text block. Default model claude-sonnet-5-5.
- Key: `env("ANTHROPIC_API_KEY")`; missing → an error naming the variable. A wrong key → the API's own reason
  (`net.post: HTTP status 401: {… "invalid x-api-key"}`): post now carries the body of a failed answer (native and browser).
- `post(url, body, headers)` of lib/net.warp: a map of texts as the request's headers (std_adapters header_pairs,
  host-files.js postSync).
- Browser: no proxy needed; the API answers browsers that send `anthropic-dangerous-direct-browser-access: true`, and
  the playground has no CSP. Checked headless 2026-10-09 with a wrong key (401 comes back through CORS).
- The playground key (⋯ menu, localStorage "warp-playground-anthropic-key") never reaches program code: env gives the
  stand-in "playground-key", and host-files.js withPageSecret swaps it for the key only in a header of a request to
  api.anthropic.com; any other URL is an error. So a pasted program cannot send the key away.
- assistant.js: completion (Ctrl-Space / Alt-Space, claude-haiku-5-5, gray at the cursor, Tab takes it, Esc or typing
  drops it) and Ask ✦ (claude-sonnet-5-5, the program and its last output as context, primer.md as the language
  brief, also the site's /llms.txt; code blocks get "put into the editor").
- Tests: tests/modules/test_agent.rs (native only: it sets the process environment; the live answer is checked only
  when a key is set), test_std_net post_sends_the_headers_of_a_map. samples/agent.warp catches the missing key with
  try, so it runs in every test run and stays in the playground menu.
- Open: `try` does not catch a host adapter failure (card try-catch).
