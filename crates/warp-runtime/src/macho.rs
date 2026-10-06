//! Just enough of 64-bit Mach-O (macOS executables) to carry a program so the executable still signs cleanly:
//! codesign wants every byte of the file inside a segment, its signature last. The carried program goes at the end
//! of the last segment (__LINKEDIT), whose size grows to cover it; the old signature is dropped first. Anything that
//! is not a 64-bit little-endian Mach-O (Linux's ELF) passes unchanged.
use std::ops::Range;

/// Enough of an executable's start to hold its header and load commands
pub const MAX_HEADER_BYTES: usize = 1 << 16;
const MAGIC_64: u32 = 0xfeed_facf;
const HEADER_BYTES: usize = 32;
const COMMAND_COUNT_AT: usize = 16;
const COMMANDS_SIZE_AT: usize = 20;
const SEGMENT_64: u32 = 0x19;
const CODE_SIGNATURE: u32 = 0x1d;
/// segment_command_64: vmsize, fileoff, filesize follow the name and vmaddr
const SEGMENT_VMSIZE_AT: usize = 32;
const SEGMENT_FILEOFF_AT: usize = 40;
const SEGMENT_FILESIZE_AT: usize = 48;
/// linkedit_data_command: dataoff after cmd and cmdsize
const DATA_OFFSET_AT: usize = 8;
/// a segment's memory size is whole pages: arm64's 16 KB, a multiple of x86-64's 4 KB
const PAGE_BYTES: u64 = 0x4000;

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
	Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
	Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

fn set_u32(bytes: &mut [u8], at: usize, value: u32) {
	bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn set_u64(bytes: &mut [u8], at: usize, value: u64) {
	bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

/// The load commands of a 64-bit Mach-O: (command, where it lies); none for any other file
fn load_commands(bytes: &[u8]) -> Vec<(u32, Range<usize>)> {
	let mut commands = Vec::new();
	if u32_at(bytes, 0) != Some(MAGIC_64) {
		return commands;
	}
	let count = u32_at(bytes, COMMAND_COUNT_AT).unwrap_or(0);
	let mut at = HEADER_BYTES;
	for _ in 0..count {
		let (Some(command), Some(size)) = (u32_at(bytes, at), u32_at(bytes, at + 4)) else { break };
		commands.push((command, at..at + size as usize));
		at += size as usize;
	}
	commands
}

/// Where the code signature starts, if the executable is a signed Mach-O
pub fn signature_offset(bytes: &[u8]) -> Option<usize> {
	let (_, signature) = load_commands(bytes).into_iter().find(|(command, _)| *command == CODE_SIGNATURE)?;
	u32_at(bytes, signature.start + DATA_OFFSET_AT).map(|offset| offset as usize)
}

/// The executable without its code signature: the signature's bytes and its load command removed
pub fn unsigned(bytes: &[u8]) -> Vec<u8> {
	let mut image = bytes.to_vec();
	let Some((_, signature)) = load_commands(bytes).into_iter().find(|(command, _)| *command == CODE_SIGNATURE) else { return image };
	let offset = signature_offset(bytes).expect("a signature command names its offset");
	image.truncate(offset);
	let commands_end = HEADER_BYTES + u32_at(bytes, COMMANDS_SIZE_AT).unwrap_or(0) as usize;
	let removed = signature.len();
	image.copy_within(signature.end..commands_end, signature.start);
	image[commands_end - removed..commands_end].fill(0);
	set_u32(&mut image, COMMAND_COUNT_AT, u32_at(bytes, COMMAND_COUNT_AT).unwrap_or(1) - 1);
	set_u32(&mut image, COMMANDS_SIZE_AT, (commands_end - HEADER_BYTES - removed) as u32);
	extend_last_segment_to_end(&mut image);
	image
}

/// Make the segment that ends the file (__LINKEDIT) reach the end of the file: what was appended becomes its content
pub fn extend_last_segment_to_end(image: &mut [u8]) {
	let last_segment = load_commands(image).into_iter()
		.filter(|(command, _)| *command == SEGMENT_64)
		.max_by_key(|(_, segment)| u64_at(image, segment.start + SEGMENT_FILEOFF_AT));
	let Some((_, segment)) = last_segment else { return };
	let file_offset = u64_at(image, segment.start + SEGMENT_FILEOFF_AT).unwrap_or(0);
	let file_size = image.len() as u64 - file_offset;
	set_u64(image, segment.start + SEGMENT_FILESIZE_AT, file_size);
	set_u64(image, segment.start + SEGMENT_VMSIZE_AT, file_size.next_multiple_of(PAGE_BYTES));
}
