#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod model;
mod providers;
mod runtime;
mod minibar;
mod updates;

fn main() { runtime::run(); }
