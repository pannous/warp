// card web-host-parts-std: std_pure and std_io carry every std module's words, so the emitter names the modules a
// program calls in its custom section warp.std_modules and a site ships the parts of those modules only: a page
// keeping values or a table gets the store (host-files.js) without the sound and MIDI code (host-sound.js)
use super::test_host_parts::part_files;

fn std_modules(code: &str) -> Vec<String> {
	let module = warp::pipeline::for_a_page(|| warp::pipeline::compile(code)).expect("compiled");
	warp::host::host_parts::std_modules_of(&module.bytes).expect("the std modules")
}

#[test]
fn the_module_names_the_std_modules_it_calls() {
	assert_eq!(std_modules("stored theme = \"dark\"\np{ theme }"), ["store"]);
	assert_eq!(std_modules("p{ \"hello\" }"), Vec::<String>::new());
}

#[test]
fn a_page_keeping_values_ships_no_sound() {
	assert_eq!(part_files("stored theme = \"dark\"\np{ theme }"), ["host-files.js"]);
}

#[test]
fn a_page_with_a_table_ships_no_sound() {
	let parts = part_files("class User{name: text}\nstored users: [User] in \"users_parts\"\np{ count(users) }");
	assert!(parts.contains(&"host-files.js") && !parts.contains(&"host-sound.js"), "{parts:?}");
}

#[test]
fn a_page_playing_a_file_ships_the_sound_part() {
	let parts = part_files("use sound\nplay \"song.mp3\"\np{ \"playing\" }");
	assert!(parts.contains(&"host-sound.js") && parts.contains(&"host-files.js"), "{parts:?}");
}
