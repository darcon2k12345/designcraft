//! The launcher's logic, shared by the window (`patronus-launcher`) and the console tool
//! (`patronus-launcher-cli`). No user interface lives here.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod core;
pub mod plan;

pub use core::{Action, Launcher, Tone, View, describe};
