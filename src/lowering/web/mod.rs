//! Web: pages, markup, styles, element events, routes and serving, components and their state

// the passes here name their siblings in src/lowering as `super::x`, as they did before the folder
#[allow(unused_imports)]
use super::*;

pub mod component_state;
pub mod component_worlds;
pub mod element_events;
pub mod field_elements;
pub(crate) mod listeners;
pub mod markup_tags;
pub mod page_html;
pub mod routes;
pub mod serve;
