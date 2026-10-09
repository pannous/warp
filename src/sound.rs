//! sound_samples(samples, count, rate) natively (card basic-sound, lib/sound.warp): 16-bit mono samples as a WAV file in the
//! system's temporary folder (warp-sound/sound.wav, then sound-2.wav … within one run, as paint's PNGs), played by the
//! system's player when the warp binary may reach the user (paint::shows_windows: never under WARP_NO_WINDOW, CI or
//! tests, which only get the file). The playground plays the same samples with WebAudio (host.js sound_samples).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The samples cross as whole numbers ≥ 0: amplitude + 32768 (lib/sound.warp sample_offset)
pub const SAMPLE_OFFSET: i64 = 32_768;
const FOLDER: &str = "warp-sound";
const FILE_STEM: &str = "sound";
/// macOS, then PulseAudio and ALSA on Linux
const PLAYERS: [&str; 3] = ["afplay", "paplay", "aplay"];
const PCM_FORMAT: u16 = 1;
const CHANNELS: u16 = 1;
const BITS_PER_SAMPLE: u16 = 16;
const BYTES_PER_SAMPLE: u32 = (BITS_PER_SAMPLE / 8) as u32;
const FORMAT_CHUNK_SIZE: u32 = 16;
const HEADER_SIZE_AFTER_RIFF: u32 = 36;
/// The sound calls of this run so far
static SOUNDED: AtomicUsize = AtomicUsize::new(0);

/// Write the samples as a WAV and play it when the user may hear it, else say where it is; played to its end, so
/// sounds one after another play in order
pub fn sound(samples: &[u64], rate: u32) -> Result<PathBuf, String> {
	let path = wav_file(samples, rate)?;
	if crate::paint::shows_windows() {
		play(&path)?;
	} else {
		eprintln!("sound {:.2} s: {}", samples.len() as f64 / rate.max(1) as f64, path.display());
	}
	Ok(path)
}

fn wav_file(samples: &[u64], rate: u32) -> Result<PathBuf, String> {
	let folder = std::env::temp_dir().join(FOLDER);
	std::fs::create_dir_all(&folder).map_err(|failure| format!("sound: cannot create {}: {failure}", folder.display()))?;
	let call = SOUNDED.fetch_add(1, Ordering::Relaxed) + 1;
	let path = folder.join(if call == 1 { format!("{FILE_STEM}.wav") } else { format!("{FILE_STEM}-{call}.wav") });
	std::fs::write(&path, wav(samples, rate)).map_err(|failure| format!("sound: cannot write {}: {failure}", path.display()))?;
	Ok(path)
}

/// The first player the system has plays the file to its end
fn play(path: &Path) -> Result<(), String> {
	for player in PLAYERS {
		if let Ok(status) = std::process::Command::new(player).arg(path).status() {
			return if status.success() { Ok(()) } else { Err(format!("sound: {player} could not play {}", path.display())) };
		}
	}
	Err(format!("sound: no player found ({}) for {}", PLAYERS.join(", "), path.display()))
}

/// A 16-bit mono PCM WAV of the offset samples
pub fn wav(samples: &[u64], rate: u32) -> Vec<u8> {
	let data: Vec<u8> = samples.iter().flat_map(|&sample| ((sample as i64 - SAMPLE_OFFSET).clamp(i16::MIN as i64, i16::MAX as i64) as i16).to_le_bytes()).collect();
	let format = [
		PCM_FORMAT.to_le_bytes().as_slice(), &CHANNELS.to_le_bytes(), &rate.to_le_bytes(), &(rate * BYTES_PER_SAMPLE * CHANNELS as u32).to_le_bytes(),
		&(BYTES_PER_SAMPLE as u16 * CHANNELS).to_le_bytes(), &BITS_PER_SAMPLE.to_le_bytes(),
	].concat();
	[
		b"RIFF".as_slice(), &(HEADER_SIZE_AFTER_RIFF + data.len() as u32).to_le_bytes(), b"WAVE",
		b"fmt ", &FORMAT_CHUNK_SIZE.to_le_bytes(), &format,
		b"data", &(data.len() as u32).to_le_bytes(), &data,
	].concat()
}
