// The `ack:` lines of the `.warp-answers` file earlier versions wrote are adopted by `.warp-acknowledged`, once
use warp::diagnostic::adopt_acknowledgements;

const DIRECTORY: &str = "scratch/test_adopted_acknowledgements";

#[test]
fn acknowledgements_of_the_old_answers_file_are_adopted_once() {
	std::fs::create_dir_all(DIRECTORY).unwrap();
	let (old, new) = (format!("{DIRECTORY}/.warp-answers"), format!("{DIRECTORY}/.warp-acknowledged"));
	std::fs::write(&old, "ack:slash-comment = acknowledged\nsomething: else\n").unwrap();
	let _ = std::fs::remove_file(&new);
	adopt_acknowledgements(&old, &new);
	adopt_acknowledgements(&old, &new);
	assert_eq!(std::fs::read_to_string(&new).unwrap(), "ack:slash-comment = acknowledged\n");
}
