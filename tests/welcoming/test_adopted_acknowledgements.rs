// The `ack:` lines of the `.wasp-answers` file earlier versions wrote are adopted by `.wasp-acknowledged`, once
use warp::diagnostic::adopt_acknowledgements;

const DIRECTORY: &str = "scratch/test_adopted_acknowledgements";

#[test]
fn acknowledgements_of_the_old_answers_file_are_adopted_once() {
	std::fs::create_dir_all(DIRECTORY).unwrap();
	let (old, new) = (format!("{DIRECTORY}/.wasp-answers"), format!("{DIRECTORY}/.wasp-acknowledged"));
	std::fs::write(&old, "ack:slash-comment = acknowledged\nsomething: else\n").unwrap();
	let _ = std::fs::remove_file(&new);
	adopt_acknowledgements(&old, &new);
	adopt_acknowledgements(&old, &new);
	assert_eq!(std::fs::read_to_string(&new).unwrap(), "ack:slash-comment = acknowledged\n");
}
