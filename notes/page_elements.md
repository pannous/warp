# Page ids as names: `who`, `$who`, `element(x)` (decision page-ids)

```warp
greeting = "Type a name"
div{
	input{ id: "who" }
	button{ on click { greeting = "Hello " + $who + ", from " + document.title; who.focus() } "Greet" }
	p{ greeting }
}
```

Pass: src/lowering/foreign/page_names.rs, right before foreign_modules (src/pipeline.rs), after markup_tags made
every element `tag{…}`.

- Ids: `id: "who"` (or `id: who` of a name the program doesn't define) of any element tag in the program. A defined
  name computes the id: `element(name)`.
- Plain `who` in code → `document.getElementById("who")`. Not: attribute and data key names (left of `:`), member
  names (`x.who`), method names. `document.x`, `window.x` → an implied `use js` (as a receiver only: `window(xs, 3)`
  of `use list` stays the word).
- `$who` → the value: `.checked` of a checkbox/radio input, `.valueAsNumber` of a number/range input, `.value` of
  input/textarea/select, `.textContent` otherwise. `$who = v` (and compound) writes that member.
- `element(x)` → `document.getElementById(x)` unless the program defines `element`.
- Clash: an id that is a variable or function of the program is always a compile-time error naming both; for a
  variable of a field (`input{ id: "name" }` + `name = $name`, the common case) it names `input{ bind: name }`. A
  library word (map, sort, print…) clashes only when the program uses the plain name: HTML pages are full of ids
  like `map` or `search` that never meet the word.
- Bind hint: the first `$who` read of a field per program advises `input{ bind: who }` (normalize::advise, got-it).
- Old forms, one release with a hint: `$who.value` → `$who`, `$who.focus()` → `who.focus()`, `$document.title` →
  `document.title`, `$xyz.x` of no id → `element("xyz").x`, `$Math.floor` → `Math`.
- jQuery: `$('#who')` parses as the call of `$` (warp_parser/atoms.rs) and is the element, with
  advice naming `who` (`$('#who').value` → `$who`; no markup id → `element("who")`); another selector is the error
  naming `document.querySelector(…)`.
- Untouched: texts (markup text, single-quoted strings: `$` means something only in warp code), bare `$main` of no
  id (tests/parser/test_dollar_names.rs), `$0`, `$a` naming an enclosing data node (references.rs), `data …`
  (code patterns' holes). A route's `$title` (lowering/web) and shader holes are taken before this pass.
- Open (decision): bare `$who` in markup text position as a live binding.

## In the playground's Worker

Programs there run in a Worker, which has no document. The wasm32 compiler types page-only globals (web_idl.rs
page_only) by the page's interface and the call goes to the page:

- host-foreign.js `callOnPage`: the js runtime, finding no such global in its Worker (or given a `PageObject`), asks the
  page synchronously (host-tasks.js askPage: a shared buffer + Atomics.wait, needs cross-origin isolation).
- task-workers.js `answerPageCall` on the page: js-members.js `jsMember` (shared with the Worker's js runtime) does the
  call; a plain value comes back as it is, an object stays on the page in `pageObjects` and crosses as
  `{$page: id, type, text}`, a `PageObject` in the Worker, which later calls name.
- The playground shows a program's markup in a shadow root: `document.getElementById` from a Worker looks there first
  (`self.programElements`, playground.js). Built sites and `warp app` windows run the same lowering
  (site-thread.js answers page calls).
- localStorage/sessionStorage stay refused with their warp alternative (`local[k]`, `session[k]`).

Freeing: the Worker keeps one `PageObject` per id (a WeakRef) and counts how often it got the
id; a FinalizationRegistry posts `{releasePageObject: {id, received}}` once it is collected, which serveTaskWorkers
takes. The page counts per Worker how often it gave each id and drops the object when every holder released it or
was terminated (Stop, a new run's Worker). A released id asked again says so ("no longer keeps"). Check:
`node --expose-gc probes/page_objects_release.mjs` (both sides cut from the real files).

Limits: a callback (a warp function given to the page, `addEventListener`) cannot cross.

Checks: tests/ffi/test_page_ids.rs, tests/ffi/test_browser_dollar_names.rs (old forms), tests/ffi/test_web_idl.rs
(browser suite: window and document from the Worker), the tour check "(a handler reads the value of the page's
element of an id: $who)", samples/web/page_elements.warp, wiki/html.md "Elements by id".
