// One crate, two fixtures (decision: core module first, components wait): ../rust_demo.wasm, a core module of plain
// number exports that `use x.wasm` links, and ../rust_demo.component.wasm, the WIT world of wit/demo.wit (feature
// `component`), for when warp links components
#[cfg(feature = "component")]
mod component;

pub fn fibonacci(n: u32) -> u64 {
	(0..n).fold((0u64, 1u64), |(a, b), _| (b, a + b)).0
}

#[cfg(not(feature = "component"))]
#[no_mangle]
pub extern "C" fn fib(n: u32) -> u64 {
	fibonacci(n)
}
