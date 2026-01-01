#[test]
fn test_host_integration() {
    #[cfg(feature = "WASMTIME")]{
        //         or
        //         WASMEDGE
        return;
    }
    #[cfg(not(feature = "WASM"))]{
        testHostDownload(); // no is!
    }
    test_getElementById();
    testDom();
    testDomProperty();
    testInnerHtml();
    testJS();
    testFetch();
    skip!(

        testCanvas(); // attribute setter missing value breaks browser
    );
}
