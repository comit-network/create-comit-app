#![warn(unused_extern_crates, rust_2018_idioms)]
#![warn(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::fallible_impl_from,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::dbg_macro
)]
#![forbid(unsafe_code)]

pub mod config;
pub mod create_comit_app;
pub mod docker;
pub mod env;
pub mod print_progress;
pub mod temp_fs;
