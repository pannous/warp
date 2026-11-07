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

`warp app [check] <file>` (lib/app/command.warp, dispatched by core/native/warp.warp): runs the program (`warp run`),
takes its printed page (lib/markup.warp page_of) and shows it in a window till it is closed (lib/webview.warp
webview_show; refused under WARP_NO_WINDOW or CI). `check` prints the text the page shows from the hidden view: the
headless test (test/web/test_webview.pairs.warp). The window shows the static page: its handlers (on click) do not
reach the program yet.
