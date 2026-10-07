// The kinds of parameters and returns came out of HashMap order: the same program gave "text + error" in one run and
// "data * text" in the next (probes/neural_net_flat.wasp). User functions are kept in name order now
use warp::wasm_emitter::eval;

#[test]
fn a_program_compiles_the_same_every_time() {
	let first = eval("probes/neural_net_flat.wasp").serialize();
	for _ in 0..5 {
		assert_eq!(eval("probes/neural_net_flat.wasp").serialize(), first);
	}
}
