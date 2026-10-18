//! Control flow: switch and match, loops, go blocks and parallel runs, test blocks, scoped handlers

// the passes here name their siblings in src/lowering as `super::x`, as they did before the folder
#[allow(unused_imports)]
use super::*;

pub mod blocks;
pub mod for_loop;
pub mod go_blocks;
pub mod parallel;
pub mod quasi_quote;
pub mod result_word;
pub mod run_time_blocks;
pub mod switch;
pub mod test_blocks;
pub mod text_patterns;
