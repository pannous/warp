//! Swizzles of a value known only at run time (decision S1): `f(p) := p.zy; f([1 2 3])` is [3 2]
use super::*;

/// swizzled(node, picks) -> the components a swizzle picks of a list of 2 to 4 numbers (one is the number, several a
/// list), null for any other node or a component beyond the list (then the field read goes on to its error)
pub const SWIZZLED: &str = "swizzled";
/// The bits of `picks` holding how many components it picks; each component's index follows in COMPONENT_BITS
const PICKS_COUNT_BITS: i64 = 3;
const COMPONENT_BITS: i64 = 2;

/// A swizzle's components (words::swizzle_components) as swizzled's `picks`
pub fn swizzle_picks(components: &[usize]) -> i64 {
	let indices = components.iter().enumerate();
	indices.fold(components.len() as i64, |picks, (at, component)| {
		picks | (*component as i64) << (PICKS_COUNT_BITS + COMPONENT_BITS * at as i64)
	})
}

impl WasmGcEmitter {
	pub(super) fn emit_swizzled(&mut self) {
		if !self.should_emit_function(SWIZZLED) {
			return;
		}
		let node = self.type_manager.node_type;
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let most_items = crate::words::SWIZZLE_LETTERS.len() as u32;
		let mut locals = vec![nullable; 1 + most_items as usize];
		locals.extend([ValType::I64, ValType::I64, ValType::I64, nullable]);
		self.runtime_function(SWIZZLED, vec![node_ref, ValType::I64], vec![nullable], locals, |s, f| {
			let (list, picks, cell, first_item) = (0, 1, 2, 3);
			let count = first_item + most_items;
			let (at, component, made) = (count + 1, count + 2, count + 3);
			let none = [I::RefNull(HeapType::Concrete(node)), I::Return];
			s.emit_field(f, list, 0);
			Self::emit_kind_test(f, Kind::List as i64, I::I64Ne);
			Self::emit_list(f, &[I::If(BlockType::Empty), none[0].clone(), none[1].clone(), I::End]);
			Self::emit_list(f, &[I::LocalGet(list), I::LocalSet(cell), I::Block(BlockType::Empty)]);
			// the items, each a number (not a bool), into their locals
			for item in first_item..first_item + most_items {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(cell), I::RefIsNull, I::BrIf(0),
					I::LocalGet(cell), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: 1 },
					I::RefCastNonNull(HeapType::Concrete(node)), I::LocalTee(item),
					I::StructGet { struct_type_index: node, field_index: 0 }, I::I64Const(UNMARKED_KIND), I::I64And,
					I::LocalTee(component),
					I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Float as i64), I::I64Eq,
					I::LocalGet(component), I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Int as i64), I::I64Eq,
					I::LocalGet(component), I::I64Const(crate::type_kinds::BOOL_KIND), I::I64Ne, I::I32And,
					I::I32Or, I::I32Eqz, I::If(BlockType::Empty), none[0].clone(), none[1].clone(), I::End,
					I::LocalGet(count), I::I64Const(1), I::I64Add, I::LocalSet(count),
					I::LocalGet(cell), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: 2 },
					I::LocalSet(cell),
				]);
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::End,
				I::LocalGet(cell), I::RefIsNull, I::I32Eqz, I::LocalGet(count), I::I64Const(2), I::I64LtS, I::I32Or,
				I::If(BlockType::Empty), none[0].clone(), none[1].clone(), I::End,
				I::LocalGet(picks), I::I64Const((1 << PICKS_COUNT_BITS) - 1), I::I64And, I::LocalSet(at),
				// the picked items from the last, each consed onto the ones after it
				I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
					I::LocalGet(at), I::I64Eqz, I::BrIf(1),
					I::LocalGet(at), I::I64Const(1), I::I64Sub, I::LocalSet(at),
					I::LocalGet(picks), I::I64Const(PICKS_COUNT_BITS), I::LocalGet(at), I::I64Const(COMPONENT_BITS),
					I::I64Mul, I::I64Add, I::I64ShrU, I::I64Const((1 << COMPONENT_BITS) - 1), I::I64And,
					I::LocalTee(component), I::LocalGet(count), I::I64GeS,
					I::If(BlockType::Empty), none[0].clone(), none[1].clone(), I::End,
					I::I64Const(SQUARE_LIST_KIND),
			]);
			// the item at `component`: nested choices of the item locals
			for item in first_item..first_item + most_items - 1 {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(component), I::I64Const((item - first_item) as i64), I::I64Eq,
					I::If(BlockType::Result(nullable)), I::LocalGet(item), I::Else,
				]);
			}
			f.instruction(&I::LocalGet(first_item + most_items - 1));
			for _ in 1..most_items {
				f.instruction(&I::End);
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
					I::LocalGet(made), I::StructNew(node), I::LocalSet(made),
					I::Br(0),
				I::End, I::End,
				// one component is the number itself
				I::LocalGet(picks), I::I64Const((1 << PICKS_COUNT_BITS) - 1), I::I64And, I::I64Const(1), I::I64Eq,
				I::If(BlockType::Result(nullable)),
					I::LocalGet(made), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: 1 },
					I::RefCastNullable(HeapType::Concrete(node)),
				I::Else, I::LocalGet(made), I::End,
			]);
		});
	}
}
