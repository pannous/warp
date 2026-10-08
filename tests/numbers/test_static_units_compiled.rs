//! Static units, stage 5: a compiled module names the unit of its result in the custom section `wasp.meta` (entry units), read back when
//! it runs; a whole list of quantities prints with its units
use warp::wasm_emitter::eval;

const LOOP: &str = "total = 0 m; for i in 1..3 { total += 250 m }; total";

#[test]
fn test_a_compiled_quantity_carries_its_unit() {
	let module = warp::pipeline::compile(LOOP).expect("compiles");
	assert!(module.bytes.windows(b"wasp.meta".len()).any(|window| window == b"wasp.meta"));
	#[cfg(feature = "native")] // the browser build runs modules in the page (test_eval_reads_the_same_section)
	assert_eq!(warp::wasm_reader::read_bytes(&module.bytes).expect("runs").serialize().trim(), "500m");
}

#[test]
fn test_eval_reads_the_same_section() {
	assert_eq!(eval(LOOP).serialize().trim(), "500m");
}

#[test]
fn test_a_whole_list_of_quantities_shows_its_units() {
	assert_eq!(eval("xs = [1 m, 250 cm]; xs").serialize(), "\"[100cm 250cm]\"");
}
