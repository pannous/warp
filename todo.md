TODO.md Usually To track open issues we're using the board 

https://github.com/users/pannous/projects/1/

If for some reason that's unreachable or GitHub is down, we can fall back to this manual file 

Here is a list of some random issues I encountered. Agent reading this: please create a proper card from them and remove todos here!

## pixels-bytes BLOCKER (2026-11-06, Grok Coder C)

Lib-only cannot land `1.bytes(big_endian)` / `1.bytes(little_endian)` (decision pixels-bytes refined).

Proved on origin/main @ 3e610d3a1 (worktree pixels-bytes):

1. **enum-overload-dispatch (core)** — a lib module overload `bytes(n: int, order: Endian)` beside prelude/builtin `bytes(t: text)` is read as `pattern·clause·bytes` (same for list.warp). A *new* name with an Endian param works in bits (`number_bytes(n, Little)` green via int_bytes). Same-file overloads in a *program* also work. Module-level enum-param overload of an existing name needs the core fix (triage enum-overload-dispatch).

2. **bare `big_endian` / `little_endian`** — those spellings are already public words of hash and sound. Bare lookup without `use` refuses with "defined by hash and sound: say which". Expected-type enum resolution does not beat that. Local program enums named that way work; module-exported ones do not. lib_consistency wants hash/sound helpers to migrate to `n.bytes(big_endian)` — that rename (and test_hash_endian edits) is outside this card's "no unasked test edits", or needs a core name-resolution tweak.

3. **prelude path** — extra `bytes(…, Endian)` lines in prelude.warp still dispatched to builtin `text_byte_list` with 2 arguments; prelude does not register the enum overloads next to the text builtin the way a program-local overload does.

Reuse target is ready once core clears (1): `int_bytes` / `float_bytes` (64-bit default) in bits. Width = 8 bytes. No core/extensions edits made; tree clean; no push.
