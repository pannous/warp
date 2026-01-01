use wasp::{eq, is};

#[test]
// fn testPaint() {
//     // #[cfg(feature = "SDL")]{
//         init_graphics();
//         while (1)
// //         paint(-1);
// //     }
// // }

// #[test]
fn test_paint_wasm() {
    #[cfg(feature = "GRAFIX")]{
        //	struct timeval stop, start;
        //	gettimeofday(&start, NULL);
        // todo: let compiler compute constant expressions like 1024*65536/4
        //    	is!("i=0;k='hi';while(i<1024*65536/4){i++;k#i=65};k[1]", 65)// wow SLOOW!!!
        //out of bounds memory access if only one Memory page!
        //         is!("i=0;k='hi';while(i<16777216){i++;k#i=65};paint()", 0) // still slow, but < 1s
        // wow, SLOWER in wasm-micro-runtime HOW!?
        //	exit(0);

        //(√((x-c)^2+(y-c)^2)<r?0:255);
        //(x-c)^2+(y-c)^2
        is!("h=100;r=10;i=100;c=99;r=99;x=i%w;y=i/h;k=‖(x-c)^2+(y-c)^2‖<r", 1);
        ////char *wasm_paint_routine = "urface=(1,2);i=0;while(i<1000000){i++;surface#i=i*(10-√i);};paint";
        //         char * wasm_paint_routine = "w=1920;c=500;r=100;surface=(1,2);i=0;"
        //         "while(i<1000000){"
        //         "i++;x=i%w;y=i/w;surface#i=(x-c)^2+(y-c)^2"
        "};paint";
        //((x-c)^2+(y-c)^2 < r^2)?0x44aa88:0xffeedd
        //char *wasm_paint_routine = "urface=(1,2);i=0;while(i<1000000){i++;surface#i=i;};paint";
        //is!(wasm_paint_routine, 0);
        //	char *wasm_paint_routine = "maxi=3840*2160/4/2;init_graphics();surface=(1,2,3);i=0;while(i<maxi){i++;surface#i=i*(10-√i);};";
        eval(wasm_paint_routine);
        //	paint(0);
        //	gettimeofday(&stop, NULL);
        //	printf!("took %lu µs\n", (stop.tv_sec - start.tv_sec) * 100000 + stop.tv_usec - start.tv_usec);
        //	printf!("took %lu ms\n", ((stop.tv_sec - start.tv_sec) * 100000 + stop.tv_usec - start.tv_usec) / 100);
        //	exit(0);
        //char *wasm_paint_routine = "init_graphics(); while(1){paint()}";// SDL bugs a bit
        //        while (1)paint(0);// help a little
    }
}

#[test]
fn test_bad_in_wasm() {
    // break immediately
    // testStringConcatWasm(); // TODO: implement
    is!("quare(3.0)", 9.); // todo groupFunctionCallPolymorphic
    // is!("global x=1+π", 1 + pi); // int 4 ƒ - TODO: implement pi constant
    // testWasmMutableGlobal(); // TODO: implement
    is!("i=0;w=800;h=800;pixel=(1 2 3);while(i++ < w*h){pixel[i]=i%2 };i ", 800 * 800);
    //local pixel in context wasp_main already known  with type long, ignoring new type group<byte>
    is!("grows:=it*2; grows 3*42 > grows 2*3", 1);
    // is there a situation where a COMPARISON is ambivalent?
    // sleep ( time > 8pm ) and shower ≠ sleep time > ( 8pm and true);
    // testNodeDataBinaryReconstruction(); // TODO: implement  y:{x:2 z:3}
    // testSmartReturnHarder(); // TODO: implement y:{x:2 z:3} can't work yet(?);
    is!("add1 x:=$0+1;add1 3",  4); // $0 specially parsed now
    is!("print 3", 3); // todo dispatch!
    is!("if 4>1 then 2 else 3", 2);

    // bad only SOMETIMES / after a while!
    is!("puts('ok');(1 4 3)#2", 4); // EXPECT 4 GOT 1n
    is!("'αβγδε'#3", 'γ'); // TODO! sometimes works!?
    is!("3 + √9",  6); // why !?!
    is!("id 3*42> id 2*3", 1);
    // testSquares(); // ⚠️ TODO: implement

    // often breaks LATER! usually some map[key] where key missing!
    // WHY do thesAe tests break in particular, sometimes?
    // testMergeOwn(); // TODO: implement
    // testEmitter(); // TODO: implement huh!?!
}
