// Card number-keyed: a map that starts empty and is filled by key is a map, as type(m) and `m is map` say
use crate::is;

#[test]
fn a_map_filled_by_key_is_a_map() {
	is!("m = {}; m[3] = 4; type(m)", "map");
	is!("m = {}; m[3] = 4; m[5] = 6; type(m)", "map");
	is!("m = {}; m[\"a\"] = 4; type(m)", "map");
	is!("m: map = {}; m[3] = 4; type(m)", "map");
	is!("m = {}; m[3] = 4; m is map", true);
	is!("m = {}; m[3] = 4; m is list", false);
}
