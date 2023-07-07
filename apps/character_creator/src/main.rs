// Hide console if release build
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod args;

use args::*;
use bevy::prelude::*;
use pikaxe_bevy::prelude::*;

fn main() {
    let args = CreatorArgs::init();

    App::new()
        .add_plugins(DefaultPlugins)
        .run();
}
