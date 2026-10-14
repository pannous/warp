// card crash-closure: a crash card names the frame that crashed, not the panic hooks above the unwind
use warp::errors::crash_card::first_warp_frame;

/// The parser crash of card crash-closure, shortened: main.rs's closed-pipe hook sits above the unwind
const BACKTRACE: &str = "   3: warp::errors::crash_card::file_card
   4: warp::errors::crash_card::install::{closure#0}
   5: <alloc::boxed::Box<dyn core::ops::function::Fn<(&std::panic::PanicHookInfo,)>> as core::ops::function::Fn<(&std::panic::PanicHookInfo,)>>::call
   6: {closure#0}
             at /Users/me/dev/angles/warp.worktrees.noindex/integrate/src/main.rs:171:3
   8: std::panicking::panic_with_hook
             at /rustc/bcf3d36c997dd9b5db4bb7f40cb91dd4cf46a305/library/std/src/panicking.rs:833:13
  11: __rustc::rust_begin_unwind
             at /rustc/bcf3d36c997dd9b5db4bb7f40cb91dd4cf46a305/library/std/src/panicking.rs:689:5
  12: core::panicking::panic_fmt
             at /rustc/bcf3d36c997dd9b5db4bb7f40cb91dd4cf46a305/library/core/src/panicking.rs:80:14
  14: <alloc::vec::Vec<char> as core::ops::index::Index<core::ops::range::RangeTo<usize>>>::index
  15: <warp::warp_parser::WarpParser>::at_pseudo_class_selector
  16: <warp::warp_parser::WarpParser>::peek_operator
  36: run_command
             at /Users/me/dev/angles/warp.worktrees.noindex/integrate/src/main.rs:516:19";

#[test]
fn crash_card_names_the_crashing_frame() {
	assert_eq!(
		first_warp_frame(BACKTRACE).as_deref(),
		Some("<warp::warp_parser::WarpParser>::at_pseudo_class_selector")
	);
}
