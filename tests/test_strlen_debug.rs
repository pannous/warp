use warp::is;

#[test]
fn test_strlen_debug() {
    is!("import strlen from \"c\"\nstrlen(\"hello\")", 5);
}
