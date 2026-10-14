//! Web: pages, markup, styles, element events, routes and serving, components and their state

// the passes here name their siblings in src/lowering as `super::x`, as they did before the folder
#[allow(unused_imports)]
use super::*;

pub mod element_events;
pub mod field_elements;
pub mod glued_attributes;
pub mod markup_tags;
pub mod page_html;
pub mod style_rules;
pub mod select_options;
pub mod routes;
pub mod serve;
pub mod listeners;
pub mod transitions;
pub mod component_state;
pub mod component_worlds;
