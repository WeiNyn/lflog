//! Macro expansion module for log pattern parsing.
//!
//! This module provides functionality to parse and expand macros in log patterns,
//! such as `{{field:datetime("%Y-%m-%d")}}` or `{{count:number}}`.

mod expander;
pub mod parser;

pub use expander::expand_macros;
pub use parser::{CustomMacro, MacroInvocation, Profile, Profiles};
