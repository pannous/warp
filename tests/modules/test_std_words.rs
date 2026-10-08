//! Standard library words no other test calls (card std-word, checked by test_std_coverage)
use crate::is;


#[test]
fn math_inverse_and_hyperbolic_words() {
	is!("use math; arc_cosine(1)", 0.0);
	is!("use math; hyperbolic_sine(0)", 0.0);
	is!("use math; hyperbolic_cosine(0)", 1.0);
	is!("use math; hyperbolic_tangent(0)", 0.0);
}

#[test]
fn draw_names_yellow_orange_purple() {
	is!("use draw; yellow == rgb(255, 215, 0)", true);
	is!("use draw; orange == rgb(255, 140, 0)", true);
	is!("use draw; purple == rgb(128, 0, 192)", true);
}

#[test]
fn an_ordered_map_keeps_insertion_order() {
	is!("use collections; m = OrderedMap(); m.b = 2; m.a = 1; join(keys(m), \",\")", "b,a");
}

#[test]
fn json_of_an_instance_given_its_classes() {
	is!("use json; class Point{x:int y:int}; to_json_of_classes(Point(1, 2), [\"Point\"])", "{\"x\":1,\"y\":2}");
}

#[test]
fn router_words_match_a_pattern_and_read_its_parameters() {
	is!("use router; route_matches(\"/users/:id:int\", \"/users/7\")", true);
	is!("use router; route_matches(\"/users/:id:int\", \"/users/me\")", false);
	is!("use router; route_parameter(\"/users/:id\", \"/users/7\", \"id\")", 7);
	is!("use router; route_parameter(\"/files/:path*\", \"/files/a/b\", \"path\")", "a/b");
}
