#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod model;
mod providers;
mod runtime;
mod minibar;

fn main() { runtime::run(); }
