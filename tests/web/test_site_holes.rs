//! Card web-fine-rest: a built site (site.js, here in `warp app`'s web view) patches only the holes of its markup after
//! a handler, as the playground does, instead of morphing the whole page·html into the root. The page's own script
//! counts the whole morphs while it clicks
const COUNTER: &str = r#"n = 0
div{ h1{ "Counter" } button{ on click { n += 1 } "add" } "clicked " + n + " times" p{ "and " + n } CLICKS }"#;
// once site.js hydrated the page (its global `site`): three clicks, then the count of morphs of the whole root
const CLICKS_SCRIPT: &str = "script{ \"const ready = setInterval(() => { if (!site) return; clearInterval(ready); let whole = 0; const morph = morphChildren; morphChildren = (shown, ...rest) => { if (shown.id === 'warp-root') whole++; return morph(shown, ...rest); }; for (let click = 0; click < 3; click++) document.querySelector('button').click(); document.body.append(' whole morphs ' + whole); }, 20)\" }";

#[test]
fn a_built_site_patches_only_its_holes() {
	let output = super::test_app_keys::checked("site_holes", &COUNTER.replace("CLICKS", CLICKS_SCRIPT));
	let text = String::from_utf8_lossy(&output.stdout);
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	assert!(text.contains("clicked 3 times") && text.contains("and 3"), "{text}");
	assert!(text.contains("whole morphs 0"), "{text}");
}
