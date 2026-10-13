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

// card help-markdown-comment-lines: a comment wrapped over several lines (the next word would not fit in 120 columns)
// is one doc; a shorter line ends its comment, so an unrelated comment above stays out
#[test]
fn a_wrapped_comment_is_the_whole_doc_of_its_word() {
	let page = standard_library_markdown();
	let exec = markdown_entry(&page, "exec(command:any)");
	assert!(exec.contains("`exec sh \"crontab -l\"`"), "{exec}");
	assert!(exec.ends_with("natively only, the browser runs no processes"), "{exec}");
	let queued = markdown_entry(&page, SOUND_QUEUED);
	assert!(queued.contains(": play returns at once"), "{queued}");
	assert!(!queued.contains("music_words.rs"), "{queued}");
	let meta = markdown_entry(&page, META_ATTRIBUTES);
	assert!(!meta.contains("CSS properties"), "{meta}");
}
