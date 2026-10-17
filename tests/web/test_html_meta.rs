// card html-meta (default A, notes/web_framework.md "Page facts"): the page's facts as keys on html, `html{ title:
// "Shop" description: "…" }`, go into its head as <title> and <meta name content>; `meta: {author: "Ann"}` names any
// other meta; an html with a head of its own gets them there. A meta element with its own attributes stays one
use warp::wasm_emitter::eval;
use warp::web::markup::to_html;

fn page(markup: &str) -> String {
	to_html(&eval(markup))
}

#[test]
fn page_facts_on_html_go_into_its_head() {
	assert_eq!(
		page(r#"html{ lang: "en" title: "Shop" description: "Fresh fruit" body{ "hi" } }"#),
		r#"<html lang="en"><head><title>Shop</title><meta name="description" content="Fresh fruit"></head><body>hi</body></html>"#
	);
	assert_eq!(
		page(r#"html{ meta: { title: "Shop" author: "Ann" } head{ link{ rel: "icon" href: "i.png" } } body{ "hi" } }"#),
		r#"<html><head><title>Shop</title><meta name="author" content="Ann"><link rel="icon" href="i.png"></head><body>hi</body></html>"#
	);
	assert_eq!(
		page(r#"html{ meta: { author: "Ann" } body{ "hi" } }"#),
		r#"<html><head><meta name="author" content="Ann"></head><body>hi</body></html>"#
	);
	assert_eq!(
		page(r#"html{ head{ meta{ charset: "utf-8" } } body{ "hi" } }"#),
		r#"<html><head><meta charset="utf-8"></head><body>hi</body></html>"#
	);
}
