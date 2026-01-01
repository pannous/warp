fn fetch(p0: &str) -> String {
    ureq::get(p0).call().unwrap().body_mut().read_to_string().unwrap()
}