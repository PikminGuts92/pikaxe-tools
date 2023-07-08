// Hide console if release build
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod args;

use args::*;
use bevy::{prelude::*, log::LogPlugin};
use pikaxe_bevy::prelude::*;

const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args = CreatorArgs::init();

    App::new()
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: format!("Milo Character Creator v{}", VERSION),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .set(LogPlugin {
                filter: "wgpu=error,character_creator=debug,grim=debug,pikaxe_bevy=debug".into(),
                ..Default::default()
            })
        )
        .add_plugin(MiloPlugin {
            ark_path: Some(args.ark_path.into()),
            default_outfit: args.default_outfit,
            ..Default::default()
        })
        .add_startup_system(init_milos)
        .run();
}

// TODO: Move to separate file?
fn init_milos(
    mut scene_events_writer: EventWriter<LoadMiloScene>,
) {
    let default_files = [
        "ui/sel_character.milo",
        "ui/metacam.milo"
    ];

    for milo_path in default_files {
        scene_events_writer.send(LoadMiloScene(milo_path.to_string()));
    }
}