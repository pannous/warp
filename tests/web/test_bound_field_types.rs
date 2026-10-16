// card guest-type (samples/web/typed_form.warp): a field bound to a declared variable sets it only to a value of its type;
// 5.1 typed into the guests field of `guests: int = 2` leaves guests 2 while the page marks the field invalid
#![cfg(feature = "native")]

const FORM: &str = "guests: int = 2\nbudget: float = 12.5\nform{ input{ bind: guests aria-label: \"guests\" } input{ bind: budget aria-label: \"budget\" } p{ \"\" + guests + \" guests, \" + budget + \" each\" } }";

fn page_after(entries: &[(&str, &str)]) -> String {
	let mut page = warp::web::headless::Page::render(FORM).expect("renders");
	for (field, text) in entries {
		page.type_into(field, text).expect("a bound field handles input");
	}
	page.text()
}

#[test]
fn a_bound_int_takes_whole_numbers_only() {
	assert!(page_after(&[("guests", "3")]).ends_with("3 guests, 12.5 each"), "{}", page_after(&[("guests", "3")]));
	assert!(page_after(&[("guests", "5.1")]).ends_with("2 guests, 12.5 each"), "{}", page_after(&[("guests", "5.1")]));
	assert!(page_after(&[("guests", "4"), ("guests", "4.5")]).ends_with("4 guests, 12.5 each"));
}

#[test]
fn a_bound_float_takes_any_number() {
	assert!(
		page_after(&[("budget", "7.25")]).ends_with("2 guests, 7.25 each"),
		"{}",
		page_after(&[("budget", "7.25")])
	);
	assert!(page_after(&[("budget", "8")]).ends_with("2 guests, 8 each"), "{}", page_after(&[("budget", "8")]));
}
