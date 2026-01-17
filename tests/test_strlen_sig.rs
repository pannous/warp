use warp::ffi::get_ffi_signature;

#[test]
fn test_strlen_sig() {
    if let Some(sig) = get_ffi_signature("strlen") {
        eprintln!("strlen signature: params={:?} results={:?}", sig.params, sig.results);
    } else {
        eprintln!("strlen NOT FOUND");
    }
}
