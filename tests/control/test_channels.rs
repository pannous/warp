// Channels inside one run (P155, notes/channels.md): Go's unbuffered channel, `ch = channel()`
use crate::is;

// Go: `ch <- 42` in a goroutine, `<-ch` in main
#[test]
fn a_value_sent_in_a_task_is_received() {
	is!("ch = channel(); go { ch.send(42) }; ch.receive()", 42);
	is!("ch = channel(); go { send 7 to ch }; ch.receive() * 2", 14);
}

// Go: `for v := range ch` ends when the sender closes the channel
#[test]
fn for_over_a_channel_receives_until_it_is_closed() {
	is!("ch = channel(); go { for i in 1 to 3 { ch.send(i) }; ch.close() }; total = 0; for v in ch { total += v }; total", 6);
}

// a function given a channel sends on it
#[test]
fn a_channel_passed_to_a_function() {
	is!("produce(c) := { for i in 1 to 4 { c.send(i) }; c.close() }; ch = channel(); go produce(ch); n = 0; for v in ch { n += 1 }; n", 4);
}

// Go: "all goroutines are asleep - deadlock!"
#[test]
fn receiving_with_no_task_left_is_an_error() {
	crate::common::fails_with("ch = channel(); ch.receive()", "waits forever");
}

// `channel "name"` is the machine-wide channel (src/host/channels.rs) with the same words; a program hears its own sends.
// Native only: the page's BroadcastChannel does not deliver to its sender, and a waiting receive cannot yield to it
#[cfg(feature = "native")]
#[test]
fn a_machine_channel_has_the_same_words() {
	is!("chat = channel \"warp-test-p155\"; chat.send(\"hi\"); chat.receive()", "hi");
	is!("chat = channel \"warp-test-p155-to\"; send 5 to chat; chat.receive() + 1", 6);
}

// Go panics on a send to a closed channel and on a second close; a task still sending when the program ends stops
#[test]
fn closed_channels_and_tasks_left_waiting() {
	crate::common::fails_with("ch = channel(); ch.close(); ch.send(1)", "send on a closed channel");
	crate::common::fails_with("ch = channel(); ch.close(); ch.close()", "close of a closed channel");
	is!("ch = channel(); go { ch.send(1) }; 5", 5);
}

// more tasks than Workers (the playground's pool holds 4): the sender waits its turn like natively queued tasks, and
// the receiver meets it; run inline it could never answer (card primer-channel-flake)
#[test]
fn a_sender_queued_behind_busy_tasks_is_received() {
	is!("for k in 1 to 6 { go { sleep(300 ms) } }; ch = channel(); go { ch.send(5) }; ch.receive()", 5);
	is!("jobs = []; for k in 1 to 6 { jobs.add(go { sleep(100 ms); k }) }; ch = channel(); go { for k in 1 to 3 { ch.send(k) }; ch.close() }; received = 0; for v in ch { received += v }; [sum(await all jobs), received]", warp::ints(vec![21, 6]));
}
