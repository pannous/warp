use warp::is;

#[test]
fn test_fd_write() {
	// Simple WASI puts test - just write to stdout
	is!("puts 'ok'", 0); // puts returns 0 on success
}

#[test]
fn test_wasi_puti() {
	is!("puti 56", 56); // puti returns the value that was printed
	is!("putl 56", 56); // putl is alias for puti
}

#[test]
fn test_wasi_putf() {
	is!("putf 3.1", 0); // putf returns 0
}

#[test]
#[ignore = "fd_write with variables needs more work"]
fn test_fd_write_raw() {
	// built-in wasi function with raw memory layout
	// is!("x='hello';fd_write(1,x,1,8)", 0); // needs string->iovec conversion
}
