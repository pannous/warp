# `use js` (card js-webidl-bindings, decision js-webidl-bindings)

`use js Name` makes a JavaScript global of the page reachable from warp. It works in the playground only: the native
warp has no JavaScript, and a run there stops at "the host word foreign_call is not in warp yet".

- extensions/js_globals.warp is a pass before rules_rewritten. It takes out the `use js` lines and lowers each use
  to the host word `foreign_call(runtime, receiver, member, called, arguments)`, which web/playground/host-foreign.js
  answers:
  - `Name.member(a)` becomes `("js", "Name", "member", 1, [a])`; a read `Name.member` becomes `(…, 0, [])`.
  - `x.member = v` calls the member `"set member"`.
  - `Name(a)` is the constructor (member ""). A function of the global scope, such as `atob(t)`, is called on
    `globalThis` instead.
- Types come from lib/web.webidl, read by extensions/js_globals/webidl.warp: interfaces, mixins, namespaces and
  partials, with parents and `includes`.
  - A global is its own definition (URL, console) or an attribute of the global scope's interfaces (crypto: Crypto).
  - A member its interface lacks is a compile error: `js: URL has no pathnam (lib/web.webidl)`.
  - A value of a defined interface keeps its type in a variable (`u = URL(…)`, `u.pathname`) and in a chain
    (`document.body.innerHTML`).
  - A text or a number comes back as warp's own value, so its members are warp's: `crypto.randomUUID().upper()`.
  - A global WebIDL doesn't define (Math, JSON) is reached unchecked, and so is what it gives.
- Not done yet: npm modules (`use js "path"`, which the Rust runner did through node), and handles passed as
  arguments of other calls.
