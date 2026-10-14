//! card units-text: a conversion joins a text in its target unit, `"total: " + (total as km)`; the imperial units mi,
//! ft, yd, inch (`in` stays the word of `x in xs`), lb and the alias mph (mi/h)
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn a_conversion_joins_a_text_in_its_unit() {
	assert_eq!(shown("total = 1500 m\n\"total: \" + (total as km)"), "\"total: 1.5km\"");
	assert_eq!(shown("total = 3 km\n\"total: \" + (total as m) + \"!\""), "\"total: 3000m!\"");
	assert_eq!(shown("v = 36 km/h\n\"v: \" + (v in m/s)"), "\"v: 10m/s\"");
	assert_eq!(shown("d = 0 mi\nfor i in 1..3 { d += 1 mi }\n\"d: \" + (d as km)"), "\"d: 3.218688km\"");
}

#[test]
fn imperial_units_convert_exactly() {
	assert_eq!(shown("1 mi as m"), "1609.344m");
	assert_eq!(shown("1 mile in feet"), "5280ft");
	assert_eq!(shown("1 ft + 1 inch"), "13inch");
	assert_eq!(shown("3 yd as ft"), "9ft");
	assert_eq!(shown("1 lb as g"), "453.59237g");
	assert_eq!(eval("quantity(2, \"mi\").in(\"m\")"), eval("3218.688"));
}

#[test]
fn mph_is_miles_per_hour() {
	assert_eq!(shown("60 mph in km/h"), "96.56064km/h");
	assert_eq!(shown("v = 60 mph\nt = 30 min\nv * t"), "30mi");
	assert_eq!(shown("class Car{speed: mph}\nCar(30 mph).speed"), "30mi/h");
	fails_with("class Car{speed: mph}\nCar(30 km).speed", "Car.speed holds m/s");
	assert_eq!(shown("mph = 3\nmph * 2"), "6");
}
