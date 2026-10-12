// card web-database-site: `database[k]` in a built site (`warp build --site`), not only in the playground: its click
// handler keeps a value in the page's IndexedDB store (host-files.js, loadKept before the run) and reads it back
use super::test_dom_pages::title_after_clicking;

#[test]
fn a_built_site_keeps_database_values() {
	if !crate::common::chrome_runs_here(module_path!()) {
		return;
	}
	let code = "use js document\ndiv{\n\tbutton{ on click {\n\t\tdatabase[\"clicks\"] = (database[\"clicks\"] or 0) + 1\n\t\tdocument.title = \"clicks \" + str(database[\"clicks\"])\n\t} \"Count\" }\n}";
	assert_eq!(title_after_clicking("database", code, "Count"), "clicks 1");
}
