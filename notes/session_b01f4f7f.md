# Claude Conversation (b01f4f7f)

## 👤 Human

is .claudeignore ok ?

---

## 🤖 Assistant

→ `Read`: `/Users/me/dev/angles/warp/.claudeignore`

---

## 🤖 Assistant

The `.claudeignore` currently only has:

```
vendor
```

This is minimal but reasonable. The `vendor/` directory contains vendored Cargo dependencies and should definitely be ignored.

You might also consider adding:

- `target/` - Rust build artifacts (though Claude Code typically handles this well)
- `*.wasm` - Generated WASM binaries if you don't need them indexed
- `Cargo.lock` - If you don't want it searched (though it's sometimes useful)

Current setup is fine for avoiding the large vendor directory. Want me to add any of these?

---

## 👤 Human

yep

---

## 🤖 Assistant

→ `Edit`: `/Users/me/dev/angles/warp/.claudeignore`

---

## 🤖 Assistant

Done. `.claudeignore` now ignores vendor, target, wasm files, and Cargo.lock.

---

