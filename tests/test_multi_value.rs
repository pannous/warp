// WASM multi-value: runtime helpers that produce two results return both on the stack (notes/multi_value.md)
use warp::extensions::numbers::Number;
use warp::wasm_emitter::{compile, eval};
use warp::{is, Node};
use wasmparser::{CompositeInnerType, Name, NameSectionReader, Parser, Payload, TypeRef};

const BIG_DIVISION: &str = "x=123456789012345678901234567890; (x//1000000000000) + (x % 98765432109876543210)";

fn int(digits: &str) -> Node {
	Node::Number(Number::parse_integer(digits).unwrap())
}

/// Result types of the function `name` in a module, from its name section
pub(crate) fn result_count(bytes: &[u8], name: &str) -> Option<usize> {
	let (mut imported, mut type_of_function, mut result_counts) = (0u32, Vec::new(), Vec::new());
	let mut index = None;
	for payload in Parser::new(0).parse_all(bytes) {
		match payload.unwrap() {
			Payload::TypeSection(types) => {
				for group in types.into_iter().flatten() {
					for ty in group.into_types() {
						let results = match &ty.composite_type.inner {
							CompositeInnerType::Func(function) => function.results().len(),
							_ => 0,
						};
						result_counts.push(results);
					}
				}
			}
			Payload::ImportSection(imports) => {
				for import in imports.into_imports().flatten() {
					imported += matches!(import.ty, TypeRef::Func(_)) as u32;
				}
			}
			Payload::FunctionSection(functions) => type_of_function = functions.into_iter().flatten().collect(),
			Payload::CustomSection(section) if section.name() == "name" => {
				let names = NameSectionReader::new(wasmparser::BinaryReader::new(section.data(), section.data_offset()));
				for subsection in names.flatten() {
					if let Name::Function(map) = subsection {
						index = map.into_iter().flatten().find(|naming| naming.name == name).map(|naming| naming.index);
					}
				}
			}
			_ => {}
		}
	}
	let defined = index? - imported;
	result_counts.get(*type_of_function.get(defined as usize)? as usize).copied()
}

#[test]
fn test_divmod_returns_quotient_and_remainder() {
	let module = compile(BIG_DIVISION).unwrap_or_else(|error| panic!("{error:?}"));
	assert_eq!(result_count(&module.bytes, "mag_divmod"), Some(2));
}

#[test]
fn test_big_division_results_unchanged() {
	is!("123456789012345678901234567890//1000000000000", int("123456789012345678"));
	is!("-123456789012345678901234567890//1000000000000", int("-123456789012345679"));
	is!("123456789012345678901234567890 % -1000", 890);
	is!("-123456789012345678901234567890 % -1000", 110);
	is!(BIG_DIVISION, int("60308641996265432088"));
	is!("98765432109876543210 % 98765432109876543210", 0);
	is!("12 % 98765432109876543210", 12);
}

#[cfg(feature = "optimizer")]
#[test]
fn test_optimized_multi_value_division() {
	use warp::wasm_optimizer::{OptimizationMode, WasmOptimizer};
	if !WasmOptimizer::tools_available() {
		eprintln!("Skipping: wasm-opt not found - install binaryen");
		return;
	}
	let module = compile(BIG_DIVISION).unwrap_or_else(|error| panic!("{error:?}"));
	let optimized = WasmOptimizer::library(OptimizationMode::Speed).optimize(&module.bytes).unwrap();
	assert_eq!(warp::wasm_reader::read_bytes(&optimized).unwrap(), int("60308641996265432088"));
}

#[test]
fn test_int_divmod_returns_both_results() {
	let module = compile(BIG_DIVISION).unwrap_or_else(|error| panic!("{error:?}"));
	assert_eq!(result_count(&module.bytes, "int_divmod_slow"), Some(2));
}

#[test]
fn test_big_exact_division_results_unchanged() {
	is!("123456789012345678901234567890//-1000", int("-123456789012345678901234567"));
	is!("-123456789012345678901234567890//-1000", int("123456789012345678901234568"));
	is!("123456789012345678901234567890//98765432109876543210", 1249999988);
	is!("-123456789012345678901234567890//98765432109876543210", -1249999989);
	is!("864197523086419752308641975230 / 7", int("123456789012345678901234567890"));
	is!("x = 123456789012345678901234567891 / 2; x * 2", int("123456789012345678901234567891"));
	is!("(123456789012345678901234567891 / 2) * 4 / 2", int("123456789012345678901234567891"));
	is!("98765432109876543210 / 0 > 10^40", true);
	is!("x = 98765432109876543210 / 3; x * 3", int("98765432109876543210"));
}
