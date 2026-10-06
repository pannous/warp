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

// `channel "name"` is the machine-wide channel (src/channels.rs) with the same words; a program hears its own sends
#[test]
fn a_machine_channel_has_the_same_words() {
	is!("chat = channel \"warp-test-p155\"; chat.send(\"hi\"); chat.receive()", "hi");
	is!("chat = channel \"warp-test-p155-to\"; send 5 to chat; chat.receive() + 1", 6);
}
