//! Rooms for texts that grow (card self-byte-buffer): text_concat grows a text in place when it ends where the heap
//! starts, which holds for one text built at a time. A long text copied to grow gets as much room again behind it, and
//! a room remembers where its text ends and where the room ends, so texts built in turn (a lexer's token, its output)
//! each grow into their own room: linear, not quadratic. Only a text ending exactly where its room is used up grows
//! there, so a text read before the growth keeps its bytes, as at the heap top.
//! The rooms are a table in linear memory, made behind the first room: per room two i32, used and end.
use crate::wasm_emitter::WasmGcEmitter;
use wasm_encoder::*;
use Instruction as I;

/// Rooms remembered at once, the oldest given up for a new one (a power of two); texts built in turn beyond this many
/// copy again
const ROOMS: i32 = 8;
const ROOM_BYTES: i32 = 8;
/// A shorter text is copied without a room: copying it costs no more than the room would save
const SHORTEST_WITH_ROOM: i32 = 64;
/// Where a room's text ends, where the room ends (the table needs no alignment)
const USED: MemArg = MemArg { offset: 0, align: 0, memory_index: 0 };
const END: MemArg = MemArg { offset: 4, ..USED };

impl WasmGcEmitter {
	/// The globals of the rooms: the table's address (0 before the first room), then the next room to give a new text
	fn text_room_globals(&mut self) -> (u32, u32) {
		let table = match self.text_room_globals {
			Some(table) => table,
			None => {
				let table = self.declare_global_of(ValType::I32, ConstExpr::i32_const(0));
				self.declare_global_of(ValType::I32, ConstExpr::i32_const(0));
				self.text_room_globals = Some(table);
				table
			}
		};
		(table, table + 1)
	}

	/// If the left text (local `left` its address, `left_length` its length) ends where a room is used up and the
	/// right text's bytes fit into that room, copy them there and branch out by `depth`; local `room` is scratch. An
	/// empty left text there (a slice at a room's end) grows there too: the room's own text keeps its bytes
	pub(in crate::wasm_emitter) fn emit_grow_in_room(
		&mut self,
		func: &mut Function,
		left: u32,
		left_length: u32,
		right_length: u32,
		room: u32,
		depth: u32,
	) {
		let (table, _) = self.text_room_globals();
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::GlobalGet(table), I::LocalTee(room), I::If(BlockType::Empty),
				I::Loop(BlockType::Empty),
					I::LocalGet(left), I::LocalGet(left_length), I::I32Add, I::LocalGet(room), I::I32Load(USED),
					I::I32Eq, I::LocalGet(room), I::I32Load(USED), I::LocalGet(right_length), I::I32Add,
					I::LocalGet(room), I::I32Load(END), I::I32LeU, I::I32And, I::If(BlockType::Empty),
						I::LocalGet(room), I::I32Load(USED),
		]);
		self.emit_text_field(func, 1, 0);
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(right_length), I::MemoryCopy { src_mem: 0, dst_mem: 0 },
			I::LocalGet(room), I::LocalGet(room), I::I32Load(USED), I::LocalGet(right_length), I::I32Add,
			I::I32Store(USED),
			I::Br(depth + 3),
			I::End,
			I::LocalGet(room), I::I32Const(ROOM_BYTES), I::I32Add, I::LocalTee(room), I::GlobalGet(table),
			I::I32Const(ROOMS * ROOM_BYTES), I::I32Add, I::I32LtU, I::BrIf(0),
			I::End,
			I::End,
		]);
	}

	/// The text just copied to `address` with `length` bytes, the heap starting after it: a long one gets as much room
	/// again behind it, in the next room; local `room` is scratch
	pub(in crate::wasm_emitter) fn emit_room_behind(
		&mut self,
		func: &mut Function,
		address: u32,
		length: u32,
		room: u32,
	) {
		let (table, next) = self.text_room_globals();
		let heap = self.text_heap_global.expect("emit_text_heap_global before rooms");
		let bump_heap = |amount: u32| [I::GlobalGet(heap), I::LocalGet(amount), I::I32Add, I::GlobalSet(heap)];
		Self::emit_list(
			func,
			&[I::LocalGet(length), I::I32Const(SHORTEST_WITH_ROOM), I::I32GeU, I::If(BlockType::Empty)],
		);
		self.emit_memory_room(func, heap, length);
		Self::emit_list(func, &bump_heap(length));
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::GlobalGet(table), I::I32Eqz, I::If(BlockType::Empty),
				I::I32Const(ROOMS * ROOM_BYTES), I::LocalSet(room),
		]);
		self.emit_memory_room(func, heap, room);
		Self::emit_list(func, &[I::GlobalGet(heap), I::GlobalSet(table)]);
		Self::emit_list(func, &bump_heap(room));
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::End,
			I::GlobalGet(table), I::GlobalGet(next), I::I32Const(ROOM_BYTES), I::I32Mul, I::I32Add, I::LocalTee(room),
			I::LocalGet(address), I::LocalGet(length), I::I32Add, I::I32Store(USED),
			I::LocalGet(room), I::LocalGet(address), I::LocalGet(length), I::I32Const(2), I::I32Mul, I::I32Add,
			I::I32Store(END),
			I::GlobalGet(next), I::I32Const(1), I::I32Add, I::I32Const(ROOMS - 1), I::I32And, I::GlobalSet(next),
			I::End,
		]);
	}
}
