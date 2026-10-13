use warp::std_docs::standard_library_markdown;

const META_ATTRIBUTES: &str = "html_meta_attributes";
const SOUND_QUEUED: &str = "sound_queued()";

fn markdown_entry<'a>(page: &'a str, signature: &str) -> &'a str {
	let prefix = format!("- `{signature}`");
	page.lines()
		.find(|line| line.starts_with(&prefix))
		.unwrap_or_else(|| panic!("missing standard-library entry for {signature}"))
}

#[test]
fn words_use_the_comment_line_directly_above_their_definition() {
	let page = standard_library_markdown();
	assert!(
		markdown_entry(&page, META_ATTRIBUTES).contains("a meta element's own attributes"),
		"{}",
		markdown_entry(&page, META_ATTRIBUTES)
	);
	assert!(
		markdown_entry(&page, SOUND_QUEUED).contains("wait_sound waits for them"),
		"{}",
		markdown_entry(&page, SOUND_QUEUED)
	);
}
