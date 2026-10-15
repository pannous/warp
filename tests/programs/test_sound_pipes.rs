// card sound-pipe-graph: sounds as values piped through nodes, `osc(C4, 1s) |> lowpass(800) |> delay(0.25) |> out`;
// a signal is the list of its amplitudes, a node a function of it, mix sums signals as a bus does
#![cfg(feature = "native")]
use crate::is;

const ENERGY: &str = "use sound\nenergy(signal) := sum([x * x for x in signal])\n";

#[test]
fn an_oscillator_is_a_signal_value() {
	is!("use sound\ncount(osc(440Hz, 0.1s))", 2205);
	is!("use sound\ncount(osc([C4, E4], 0.1s, \"square\"))", 2205);
}

#[test]
fn a_delay_node_adds_echoes() {
	is!("use sound\ncount(osc(440, 0.1s) |> delay(0.1, 0.5)) > 2205 + 2 * 2205", true);
}

#[test]
fn a_lowpass_node_takes_the_highs() {
	is!(&format!("{ENERGY}high = osc(5000, 0.05s)\nenergy(high |> lowpass(500)) < energy(high) / 10"), true);
	is!(&format!("{ENERGY}low = osc(100, 0.05s)\nenergy(low |> lowpass(5000)) > energy(low) / 2"), true);
}

#[test]
fn nodes_chain() {
	is!("use sound\ncount(osc(C4, 0.1s) |> distort(0.5) |> highpass(200) |> reverb(0.3) |> gain(0.5)) > 2205", true);
}

#[test]
fn a_mix_sums_signals_to_the_longest() {
	is!("use sound\ncount(mix(osc(C4, 0.1s), osc(E4, 0.2s)))", 4410);
	is!("use sound\na = osc(C4, 0.1s)\nmax(mix(a, a, a)) == 3 * max(a)", true);
}

#[test]
fn out_plays_a_signal() {
	is!("use sound\nosc(C4, 5ms) |> lowpass(800) |> out\n7", 7);
}
