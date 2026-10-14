// card midi-out: Standard MIDI Files written and read back by write_midi / read_midi (src/sound/midi.rs, host-files.js in the
// playground), and live messages, which headless (tests, CI) print as `midi out <port>: 90 3c 64` instead of sounding
#![cfg(feature = "native")]
use crate::is;
use std::path::PathBuf;

fn midi_path(name: &str) -> PathBuf {
	let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("midi");
	std::fs::create_dir_all(&folder).unwrap();
	folder.join(name)
}

/// What `program`, using sound, prints on stdout (its failure is printed there) and stderr (headless sound and MIDI lines)
fn failures_of(program: &str) -> String {
	let run =
		crate::common::warp_command().args(["--no-ask", "eval", &format!("use sound\n{program}")]).output().unwrap();
	format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr))
}

#[test]
fn notes_survive_a_midi_file() {
	let path = midi_path("tune.mid");
	is!(
		&format!(
			"use sound\nwrite_midi(\"{0}\", [[0, C4, 0.5, 100], [0.5, E4, 0.25, 90]])\nread_midi(\"{0}\") == [[0, 261.63, 0.5, 100], [0.5, 329.63, 0.25, 90]]",
			path.display()
		),
		true
	);
	let bytes = std::fs::read(&path).unwrap();
	assert_eq!(&bytes[..4], b"MThd");
	assert_eq!(&bytes[14..18], b"MTrk");
}

#[test]
fn a_file_of_another_program_reads_with_running_status_and_tempo() {
	// format 0, 96 ticks a quarter, tempo 1 s a quarter, C4 on at 0, running-status E4 on at 96, both off at 192 and 288
	let track = [
		0, 0xff, 0x51, 3, 0x0f, 0x42, 0x40, 0, 0x90, 60, 100, 96, 64, 100, 96, 0x80, 60, 0, 96, 64, 0, 0, 0xff, 0x2f, 0,
	];
	let mut file = b"MThd\0\0\0\x06\0\0\0\x01\0\x60MTrk".to_vec();
	file.extend((track.len() as u32).to_be_bytes());
	file.extend(track);
	let path = midi_path("other.mid");
	std::fs::write(&path, file).unwrap();
	is!(&format!("use sound\nread_midi(\"{}\") == [[0, 261.63, 2, 100], [1, 329.63, 2, 100]]", path.display()), true);
}

#[test]
fn a_broken_file_fails_loudly() {
	let path = midi_path("broken.mid");
	std::fs::write(&path, b"RIFFnotmidi").unwrap();
	let errors = failures_of(&format!("read_midi(\"{}\")", path.display()));
	assert!(errors.contains("not a MIDI file"), "{errors}");
}

#[test]
fn headless_messages_are_named() {
	let errors = failures_of("midi_on C4\nmidi_off(E4, 2)\nmidi_send(192, 5)\n7");
	assert!(errors.contains("midi out : 90 3c 64"), "{errors}");
	assert!(errors.contains("midi out : 81 40 00"), "{errors}");
	assert!(errors.contains("midi out : c0 05"), "{errors}");
}

#[test]
fn native_input_is_a_loud_error() {
	let errors = failures_of("midi_receive()");
	assert!(errors.contains("MIDI input is not available natively yet"), "{errors}");
	assert!(!errors.contains("the playground reads it"), "promises a way that does not exist: {errors}");
}

#[test]
fn listing_outputs_needs_a_backend_or_names_the_missing_one() {
	let output = failures_of("midi_outputs()");
	assert!(output.starts_with('»') || output.contains("sendmidi") || output.contains("Error"), "{output}");
	assert!(output.contains('[') || output.contains("sendmidi"), "{output}");
}

// card midi-clock: a message goes out at its voice's place, after the sounds queued before it or at an `at`'s time;
// headless names that place
#[test]
fn a_message_waits_for_the_sounds_before_it() {
	let printed = failures_of("play(C4, 0.5s)\nmidi_on E4\n7");
	assert!(printed.contains("midi out : 90 40 64 at 0.500 s"), "{printed}");
}

#[test]
fn at_places_a_message() {
	let printed = failures_of("at 1s midi_on C4\nmidi_on E4\n7");
	assert!(printed.contains("midi out : 90 3c 64 at 1.000 s"), "{printed}");
	assert!(printed.contains("midi out : 90 40 64\n"), "the voice goes on where it stood: {printed}");
}

#[test]
fn midi_notes_follow_one_another() {
	let printed = failures_of("midi_note(C4, 0.5s)\nmidi_note(E4, 250ms, 90)\nsound_queued() > 0");
	let lines: Vec<&str> = printed.lines().filter(|line| line.starts_with("midi out")).collect();
	assert_eq!(
		lines,
		[
			"midi out : 90 3c 64",
			"midi out : 80 3c 00 at 0.500 s",
			"midi out : 90 40 5a at 0.500 s",
			"midi out : 80 40 00 at 0.750 s"
		],
		"{printed}"
	);
}
