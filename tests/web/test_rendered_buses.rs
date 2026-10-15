// card shared-buses stage 2 in the browser: a sound sent to a bus goes through the native DSP (src/sound/effects.rs, in
// warp.wasm as web_bus_processed; host-sound.js onBus), its echoes rendered as natively: tests/programs/test_sound_buses.rs
#![cfg(not(feature = "native"))]
use crate::is;

#[test]
fn a_delay_bus_rings_on_in_the_render_as_natively() {
	is!(
		"use sound\nroom = bus() |> delay(0.3, 0.5, 1)\nosc(A4, 0.1s) |> room\nround(save(sound, \"bus.wav\") * 10)",
		29
	);
}

/// lib's list effects run the same DSP: a delay's echoes ring as long as natively (tests/programs/test_sound_pipes.rs)
#[test]
fn a_list_signal_delays_as_natively() {
	is!("use sound\ncount(osc(440, 0.1s) |> delay(0.1, 0.5))", 23105);
}
