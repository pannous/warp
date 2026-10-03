use warp::*;

#[test]
fn test_colon_body_of_while_stops_at_semicolon() {
	is!("i=1;while i<9:i++;i+1", 10);
}

#[test]
fn test_colon_body_of_while_stops_at_newline() {
	is!("i=1\nwhile i<9:i++\ni+1", 10);
}
