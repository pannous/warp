use warp::is;

#[test]
fn test_strcmp_with_assign() {
    is!("import strcmp from \"c\"\nx=strcmp(\"abc\", \"def\");x<0", 1);
}
