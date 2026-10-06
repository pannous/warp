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

## Native plan
- tasks.rs TaskTable (one per run, shared by every task thread through link_into) gets
  `channels: Mutex<HashMap<i64, Channel>>` + a Condvar; Channel { offered: Option<TaskValue>, closed: bool }.
- Host words (host.rs HOST_WORDS and signatures): channel_new() -> i64, channel_put(id, node) (blocks until taken:
  rendezvous), channel_take(id) -> node (blocks; ø once closed and empty), channel_more(id) -> i64 (blocks until a
  value waits or the channel closed: 1 / 0), channel_close(id). Values cross as TaskValue (Builders::read_value /
  Builders::build, as task_await_value and signal_send do).
- Lowering (a source pass before lower_tasks): `channel()` → channel_new(); `ch.send(v)` → channel_put(ch, v);
  `ch.receive()` → channel_take(ch); `ch.close()` → channel_close(ch); `for v in ch {…}` (ch assigned channel()) →
  `while channel_more(ch) { v = channel_take(ch); … }`. A go block capturing ch gets the id, which names the same
  channel in every task of the run.
- Machine channels: `channel "chat"` → the existing channel_listen/channel_send words behind the same methods.

## Browser
Tasks run in Workers: a channel needs a SharedArrayBuffer slot per channel and Atomics.wait. Until built, the
playground says loudly that channels need the native build (card).

Ported cases: probes/async_ports.md (the two Go channel rows).
