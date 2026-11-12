# The system's web view natively (card app-webview, decisions native-js-webview, use-js-tests)

The web view is warp (card warp-webview-removal): lib/host/native_host/webview.warp, a part of the native host module,
drives macOS's WKWebView over the Objective-C runtime by plain FFI.
- `use js` natively: the host word foreign_call is answered by the module's host_foreign_call. One hidden WKWebView per
  run, made at the first call, at warp://localhost/, a scheme the run answers itself (a secure origin: crypto.randomUUID). Never shown, activation
  policy Prohibited: no Dock icon, no focus taken, safe in tests.
- Objective-C from warp: `sent(receiver, selector, arguments)` calls objc_msgSend through foreign.c native_call (any
  signature: integers in x0…, `sent_with_floats` puts floats in d0…, so a CGRect by value is four doubles, arm64's way).
  Objects are addresses inside the module, never program handles. The classes WebKit calls back (navigation delegate,
  script message handler, window delegate) are made at run time (objc_allocateClassPair, class_addMethod) with warp
  functions as methods (lib/callbacks.warp c_callback); the completion handler is a global block built in memory whose
  invoke is a callback. A class an earlier run of the process made gets the new run's methods (class_replaceMethod).
- The call crosses as JSON (lib/json): module and member are checked as names, arguments are only JSON.
  host/native_host/webview_bridge.js after web/playground/js-members.js (one jsMember for page and native) answers it,
  the scripts read when the module is built (module_source); objects without a plain form stay in the view behind
  {$handle, type, text}, as the page's host.js handleOf does. A failure is the call's Error value.
- Lessons (cards): a lambda reading a module global keeps its value of when it was made (prebuilt-module): the states
  a run loop waits for are read through words; a list item to a C i64 goes through a float (list-item): int() first.
- A promise is awaited (callAsyncJavaScript), unlike the page, which refuses. No answer in 10 s is a loud error.
- A warp function given to js (`Array.from(xs, x => x * 2)`) is lowered to js_function(f, its parameters' count)
  (extensions/foreign_globals.warp; lib/callbacks.warp keeps it in a slot, the page's and the view's one table) and
  crosses as {$callback: slot}, which webview_bridge.js makes a JS function: each call is a synchronous XMLHttpRequest
  to warp://localhost/<slot>?<arguments as JSON> at the page's own origin (a custom scheme from https:// never reaches
  the handler), answered by the WKURLSchemeHandler while the run spins for the call's answer, with as many arguments
  as the function takes. A function whose parameters the lowering can't see (held as `any`) is refused; a slot is
  freed once its JS function is collected (FinalizationRegistry, the message warpReleased). A
  callback that calls js itself is refused: the view's one JS thread waits for it (a second view, or JavaScriptCore in
  process, would lift that).
- Not yet: warp functions given to js in the page, `use js "module"` (npm), the operator
  forwarding of host-foreign.js FOREIGN_OPERATORS, WebKitGTK on Linux.

`warp app [check] <file>` (lib/app/command.warp, dispatched by core/native/warp.warp): runs the program (`warp run`)
with --app-window, its page live in a window till it is closed (card static-page-window). After main, markup's
page_of (lib/markup.warp page_window_run) shows the run's web view in a window through `use js`'s channel
(the module answers warpWindow itself: `show`, `next`) and loops: each event of the page (webview_bridge.js,
heard by web/playground/markup.js listenToElements as the built sites' site.js hears them: click, input, change,
focus, blur, submit) runs its handler (page_event), a key the program's `on key`; `next(ms)` ticks when no event comes,
and lib/timers.warp fires its timers (`every`, `at 9:30`, `on every day at 9:30`) on each tick's local wall clock; the page rendered anew and morphed in (morphChildren).
Closing the window ends the run.
Refused under --no-window or CI. `check` prints the text the page shows from the hidden view: the headless test
(test/web/test_webview.pairs.warp). Not yet: a window test without a screen.

## Sample checks (card scripts-samples, decision sample-checks)
A sample declares its interactions: `# check: click "+1", type "name" "Ann" → count: 2`, one check a line, each from a
fresh run; the steps are lib/markup.warp's page words (click; type is type_into), the page's text, its white space
runs as one space, holds the text after →.
- natively `warp app check <file>` (lib/app/command.warp) runs the program with --page-steps (markup's
  page_after_steps does the steps before the page is printed) and reads the text in the hidden web view;
  scripts/samples.sh runs it for every sample that ran and has check lines, a failing check its FAIL
- in the page web/playground/test_in_browser.py --samples (scripts/samples-web.sh) shows the sample anew per check,
  clicks and types in the rendered DOM, and a failing check is its DIFF
