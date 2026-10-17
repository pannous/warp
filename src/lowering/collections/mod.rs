//! Collections: lists, maps, ranges, comprehensions, indexes and element-wise operations

// the passes here name their siblings in src/lowering as `super::x`, as they did before the folder
#[allow(unused_imports)]
use super::*;

pub mod broadcasting;
pub mod comprehensions;
pub mod data_items;
pub mod filled_lists;
pub mod grouped_receivers;
pub mod index_positions;
pub mod key_names;
pub mod lazy_ranges;
pub mod list_element_checks;
pub mod list_mutations;
pub mod min_max;
pub mod nested_index;
pub mod number_keys;
pub mod of_lookups;
pub mod one_entry_maps;
pub mod shared_arrays;
pub mod tree_walks;
