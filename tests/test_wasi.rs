use wasp::is;
use wasp::util::loadModule;

#[test]
fn test_fd_write() {
    // built-in wasi function
    //    is!("x='hello';fd_write(1,20,1,8)",  0);// 20 = &x+4 {char*,len}
    //    is!("puts 'ok';proc_exit(1)\nputs 'no'",  0);
    //    is!("quit",0);
    is!("x='hello';fd_write(1,x,1,8)",  0); // &x+4 {char*,len}
    //    is!("len('123')", 3); // Map::len
    //    quit();
    is!("puts 'ok'",  0); // connect to wasi fd_write
    loadModule("wasp");
    is!("puts 'ok'",  0);
    is!("puti 56", 56);
    is!("putl 56", 56);
    //    is!("putx 56", 56);
    is!("putf 3.1", 0);

    assert!(module_cache.has("wasp-runtime.wasm"s.hash()));
}
