# The system's web view natively (card app-webview, decisions native-js-webview, use-js-tests)

libwarp_webview (crates/warp-webview, built by scripts/wasmtime-c-api.sh beside warp; macOS WKWebView only so far) is a
host library, not the core binary:
- `use js` natively: the host word foreign_call is bound to its `warp_word_foreign_call` (lib/native/foreign.c
  HOST_WORD_LIBRARIES). One hidden WKWebView per run, made at the first call, at https://localhost/ (a secure origin:
  crypto.randomUUID). Never shown, activation policy Prohibited: no Dock icon, no focus taken, safe in tests.
- The call crosses as JSON (warp_json.h, plain C over lib/native/warp_node.h): module and member are checked as names,
  arguments are only JSON. bridge.js after web/playground/js-members.js (one jsMember for page and native) answers it;
  objects without a plain form stay in the view behind {$handle, type, text}, as the page's host.js handleOf does.
- A promise is awaited (callAsyncJavaScript), unlike the page, which refuses. No answer in 10 s is a loud error.
- Not yet: warp functions given to js (callbacks: `Array.from(xs, x => …)`), `use js "module"` (npm), the operator
  forwarding of host-foreign.js FOREIGN_OPERATORS, WebKitGTK on Linux.

`warp app [check] <file>` (lib/app/command.warp, dispatched by core/native/warp.warp): runs the program (`warp run`)
with WARP_APP_WINDOW set, its page live in a window till it is closed (card static-page-window). After main, markup's
page_of (lib/markup.warp page_window_run) shows the run's web view in a window through `use js`'s channel
(warp_webview.m answers the module warpWindow itself: `show`, `next`) and loops: each event of the page (bridge.js,
heard by web/playground/markup.js listenToElements as the built sites' site.js hears them: click, input, change,
focus, blur, submit) runs its handler (page_event), a key the program's `on key`; `next(ms)` ticks when no event comes,
and lib/timers.warp fires its `every` timers on each tick; the page rendered anew and morphed in (morphChildren).
Closing the window ends the run.
Refused under WARP_NO_WINDOW or CI. `check` prints the text the page shows from the hidden view: the headless test
(test/web/test_webview.pairs.warp). Not yet: clock timers (`at 9:30`) in the window, a window test without a screen.

## Sample checks (card scripts-samples, decision sample-checks)
A sample declares its interactions: `# check: click "+1", type "name" "Ann" → count: 2`, one check a line, each from a
fresh run; the steps are lib/markup.warp's page words (click; type is type_into), the page's text, its white space
runs as one space, holds the text after →.
- natively `warp app check <file>` (lib/app/command.warp) runs the program with WARP_PAGE_STEPS (markup's
  page_after_steps does the steps before the page is printed) and reads the text in the hidden web view;
  scripts/samples.sh runs it for every sample that ran and has check lines, a failing check its FAIL
- in the page web/playground/test_in_browser.py --samples (scripts/samples-web.sh) shows the sample anew per check,
  clicks and types in the rendered DOM, and a failing check is its DIFF
- found so far: page_elements fails both ways (cards bare-name: `p{ greeting }` not live; who-focus natively)
