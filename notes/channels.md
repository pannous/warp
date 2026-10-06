# Channels (P155, plan by warp-93, 2026-10-06)

User decision P155: one concept. `ch = channel()` is a channel local to the run, `channel "chat"` the machine-wide one
(src/channels.rs, sockets in /tmp/warp-channels-<user>); both have the same words, and `send v to "chat"` (P129) is the
machine channel's send.

```wasp
ch = channel()
go { for i in 1 to 3 { ch.send(i) }; ch.close() }
total = 0
for v in ch { total += v }     // 6, ends at close
ch.receive()                   // one value, waits for it
```

Semantics (Go's unbuffered channel): `send` waits until a receiver took the value; `receive` waits for a value;
`for v in ch` receives until the channel is closed and empty; a send on a closed channel is an error.

## Native (built)
- tasks.rs TaskTable (one per run, shared by every task thread) holds the channels; host words (host.rs CHANNEL_WORDS):
  channel_new() -> id, channel_put(id, node) waits until taken (rendezvous), channel_take(id) waits (ø once closed and
  empty), channel_more(id) waits until a value is offered (1) or the channel closed (0), channel_close(id). The program
  waiting with no task left is an error ("ch.receive() waits forever: no task is left to answer it"); tasks still
  waiting stop when the program ends.
- Lowering src/lowering/channel_words.rs, FIRST of SOURCE_PASSES (go_blocks renames what a go block reads,
  system_signals takes `send v to "chat"`): `channel()` → channel_new(); `ch.send(v)` / `send v to ch` →
  channel_put(ch, v); `ch.receive()` → channel_take(ch); `ch.close()` → channel_close(ch); `for v in ch {…}` →
  `while channel_more(ch) { v = channel_take(ch); … }`. Channels are the variables assigned `channel()` plus the
  parameters of functions called with one (fixed point). A go block gets the id, the same channel in every task.
- Machine channels `chat = channel "chat"`: `chat = "chat"; channel_listen(ID, "chat")` (ID from 1<<20 up, apart from
  system_signals' listener ids), `.send(v)` / `send v to chat` → channel_send(chat, v), `.receive()` polls
  channel_pending every 5 ms, then channel_next. A program hears its own sends.
- Tests: tests/control/test_channels.rs.

## Browser (built)
host.js: one SharedArrayBuffer per run (channelTable, only when the module imports channel_new and the page is
cross-origin isolated), handed to every task Worker with the shared arrays: a lock, a change counter the waiting sides
Atomics.wait on (a condition variable), the next channel id, an ended flag, then per channel [full, closed, sent,
taken, length] and a 64 KB value area (the JSON of readTaskValue's tree). Same rendezvous as natively; the program
waiting while none of its task Workers runs is the "waits forever" error, a task run inline (no free Worker) waiting
too; outcomeOf ends the channels before joining, so tasks still waiting stop. A page without isolation: channel() is
a loud error. `channel "name"` (BroadcastChannel) cannot receive in a waiting loop and does not hear its own sends:
its test is native only.
