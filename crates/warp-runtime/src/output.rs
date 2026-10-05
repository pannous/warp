//! The one WASI call warp programs make, `fd_write` (print), for a store of any state: the warp-runtime stub needs no WASI library
use wasmtime::{Caller, Extern, Linker, Result};

pub const WASI_LIBRARY: &str = "wasi_snapshot_preview1";
pub const FD_WRITE: &str = "fd_write";
const STDOUT: i32 = 1;
const STDERR: i32 = 2;
const ERRNO_SUCCESS: i32 = 0;
const ERRNO_BADF: i32 = 8;
const ERRNO_FAULT: i32 = 21;
const IOVEC_BYTES: usize = 8;

/// fd_write(fd, iovecs, count, written) to stdout or stderr
pub fn link_output<T: 'static>(linker: &mut Linker<T>) -> Result<()> {
	linker.func_wrap(WASI_LIBRARY, FD_WRITE, |mut caller: Caller<'_, T>, fd: i32, iovecs: i32, count: i32, written: i32| -> i32 {
		let Some(Extern::Memory(memory)) = caller.get_export("memory") else { return ERRNO_FAULT };
		let bytes = memory.data(&caller);
		let word = |at: usize| bytes.get(at..at + 4).map(|word| u32::from_le_bytes(word.try_into().unwrap()) as usize);
		let mut text = Vec::new();
		for index in 0..count.max(0) as usize {
			let at = iovecs as usize + index * IOVEC_BYTES;
			let (Some(start), Some(length)) = (word(at), word(at + 4)) else { return ERRNO_FAULT };
			let Some(part) = bytes.get(start..start + length) else { return ERRNO_FAULT };
			text.extend_from_slice(part);
		}
		use std::io::Write;
		let outcome = match fd {
			STDOUT => std::io::stdout().write_all(&text),
			STDERR => std::io::stderr().write_all(&text),
			_ => return ERRNO_BADF,
		};
		if outcome.is_err() {
			return ERRNO_BADF;
		}
		match memory.write(&mut caller, written as usize, &(text.len() as u32).to_le_bytes()) {
			Ok(()) => ERRNO_SUCCESS,
			Err(_) => ERRNO_FAULT,
		}
	})?;
	Ok(())
}
