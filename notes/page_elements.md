# `$name`: the page's elements and globals (card browser-interaction)

User decision (option A, via the Interviewer): `$xyz` is the element of id xyz, not its value.

```warp
greeting = "Type a name"
div{
	input{ id: "who" }
	button{ on click { greeting = "Hello " + $who.value } "Greet" }
	p{ greeting }
}
```

- `$xyz.member` → `document.getElementById("xyz").member`; `$document`, `$window`, `$Math`… (any global JavaScript
  or WebIDL declares) → that global. Each brings its `use js`, so foreign_modules types the member through WebIDL:
  `$who.vlaue` is a compile-time did-you-mean.
- Only as a member's receiver: a bare `$main` stays a symbol (tests/parser/test_dollar_names.rs). Untouched too: `$0`,
  `$a` naming an enclosing data node (references.rs), and everything inside `data …` (code patterns' holes,
  `match c { data($x.warp) => x }`, which reach this pass grouped as `(data $x.warp)`).
- Pass: src/lowering/foreign/dollar_names.rs, right before foreign_modules (src/pipeline.rs).
- A route's `$title` (a form value, lowering/web) is taken before this pass and stays.

## In the playground's Worker

Programs there run in a Worker, which has no document. The wasm32 compiler used to refuse page-only globals
(web_idl.rs page_only); now it types them by the page's interface and the call goes to the page:

- host-foreign.js `callOnPage`: the js runtime, finding no such global in its Worker (or given a `PageObject`), asks the
  page synchronously (host-tasks.js askPage: a shared buffer + Atomics.wait, needs cross-origin isolation).
- task-workers.js `answerPageCall` on the page: js-members.js `jsMember` (shared with the Worker's js runtime) does the
  call; a plain value comes back as it is, an object stays on the page in `pageObjects` and crosses as
  `{$page: id, type, text}`, a `PageObject` in the Worker, which later calls name.
- The playground shows a program's markup in a shadow root: `document.getElementById` from a Worker looks there first
  (`self.programElements`, playground.js).
- localStorage/sessionStorage stay refused with their warp alternative (`local[k]`, `session[k]`).

Limits: a callback (a warp function given to the page, `addEventListener`) cannot cross; page objects are never freed
during a page's life: the same object keeps its one entry (`pageObjectIds`), but each new one (a fresh NodeList of
every `querySelectorAll`) adds one (card page-pageobjects).

Checks: tests/ffi/test_browser_dollar_names.rs, tests/ffi/test_web_idl.rs (browser suite: window and document from the
Worker), the tour check "(a handler reads the page's element of an id: $who.value)", samples/page_elements.warp.
