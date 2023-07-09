// Hide console if release build
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod args;

use args::*;
use bevy::{prelude::*, log::LogPlugin};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use bevy_infinite_grid::{GridShadowCamera, InfiniteGridBundle, InfiniteGrid, InfiniteGridPlugin};
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
        .add_plugin(FlyCameraPlugin)
        .add_plugin(InfiniteGridPlugin)
        .add_startup_system(init_milos)
        .add_startup_system(setup)
        .add_system(control_camera)
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });

    let mut camera = Camera3dBundle::default();
    camera.transform = Transform::from_xyz(-2.0, 2.5, 5.0)
        .looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn(camera).insert(FlyCamera {
        enabled: false,
        sensitivity: 0.0,
        ..Default::default()
    }).insert(GridShadowCamera); // Fix camera

    let mut camera = Camera3dBundle::default();
    camera.camera.is_active = false;
    camera.transform = Transform::from_xyz(-2.0, 2.5, 10.0)
        .looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn(camera).insert(FlyCamera {
        enabled: false,
        sensitivity: 0.0,
        ..Default::default()
    }); // Fix camera

    // Infinite grid
    commands.spawn(InfiniteGridBundle {
        grid: InfiniteGrid {
            fadeout_distance: 300.,
            shadow_color: None, // No shadow
            ..InfiniteGrid::default()
        },
        visibility: Visibility::Visible,
        ..InfiniteGridBundle::default()
    });
}

fn control_camera(
    key_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    mut cam_query: Query<(&mut Camera, Option<&mut FlyCamera>)>,
) {
    let key_down = is_camera_button_down(&key_input);
    let mouse_down = mouse_input.pressed(MouseButton::Left);

    let count = cam_query.iter().count();
    let cycle_cam = key_input.any_just_released([KeyCode::C]);

    let current_idx = cam_query
        .iter()
        .enumerate()
        .find(|(_, (c, _))| c.is_active)
        .map(|(i, _)| i)
        .unwrap_or_default();

    let next_idx = match (cycle_cam, current_idx + 1) {
        (true, next) if next < count => next,
        (true, _) => 0,
        _ => current_idx
    };

    for (i, (mut cam, fly_cam)) in cam_query.iter_mut().enumerate() {
        cam.is_active = i == next_idx;

        if let Some(mut fly_cam) = fly_cam {
            // Disable camera move if mouse button not held
            fly_cam.sensitivity = match mouse_down {
                true => 3.0,
                _ => 0.0
            };

            fly_cam.enabled = key_down || mouse_down;
        }
    }
}

fn is_camera_button_down(key_input: &Res<Input<KeyCode>>) -> bool {
    let control_keys = [
        KeyCode::W,
        KeyCode::A,
        KeyCode::S,
        KeyCode::D,
        KeyCode::Space,
        KeyCode::LShift,
    ];

    control_keys
        .iter()
        .any(|k| key_input.pressed(*k))
}