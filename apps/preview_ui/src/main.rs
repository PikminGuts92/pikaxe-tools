#![allow(dead_code)]
#![allow(unused_imports)]

// Hide console if release build
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod events;
mod gui;
mod plugins;
mod render;
mod settings;
mod state;

use events::*;
use gui::*;
use render::{render_milo, render_milo_entry};
use settings::*;
use bevy::{ecs::world::OnDespawn, prelude::*, render::camera::{PerspectiveProjection, RenderTarget}, window::{PresentMode, PrimaryWindow, WindowClosed, WindowMode, WindowRef, WindowResized}, winit::WinitWindows};
use bevy_egui::{EguiContext, EguiContextPass, EguiPlugin, egui, egui::{Color32, Context, Pos2, Ui}};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use bevy_infinite_grid::{InfiniteGrid, InfiniteGridBundle, InfiniteGridPlugin, InfiniteGridSettings};
use pikaxe::*;
use pikaxe::ark::{Ark, ArkOffsetEntry};
use pikaxe::scene::*;
use log::{debug, info, warn};
use plugins::*;
use state::*;
use std::{env::args, path::{Path, PathBuf}};

use crate::render::open_and_unpack_milo;

#[derive(Component)]
pub struct WorldMesh {
    name: String,
    vert_count: usize,
    face_count: usize,
}

fn main() {
    App::new()
        .add_event::<AppEvent>()
        .add_event::<AppFileEvent>()
        //.insert_resource(ClearColor(Color::BLACK))
        .add_plugins(GrimPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true
        })
        .add_plugins(FlyCameraPlugin)
        .add_plugins(InfiniteGridPlugin)
        .add_systems(EguiContextPass, render_gui_system)
        .add_systems(Update, detect_meshes)
        .add_systems(Update, control_camera)
        .add_systems(Update, drop_files)
        .add_systems(Update, window_resized)
        .add_observer(window_closed)
        .add_systems(Update, consume_file_events)
        .add_systems(Update, consume_app_events)
        .add_systems(Startup, setup_args)
        .add_systems(Startup, setup)
        .run();
}

fn render_gui_system(
    mut commands: Commands,
    mut settings: ResMut<AppSettings>,
    mut state: ResMut<AppState>,
    egui_ctx_query: Query<(&mut EguiContext, &Window, Has<PrimaryWindow>)>,
    mut event_writer: EventWriter<AppEvent>) {
    let window_count = egui_ctx_query.iter().count();

    for (egui_ctx, window, is_primary_window) in egui_ctx_query {
        if is_primary_window {
            render_gui(&mut egui_ctx.get(), &mut *settings, &mut *state);
            render_gui_info(&mut egui_ctx.get(), &mut *state);
        }

        render_lower_icons(&mut egui_ctx.get(), &mut *settings, &mut *state);

        state.consume_events(|ev| {
            // TODO: Handle in another system
            if let AppEvent::CreateNewWindow = ev {
                let new_window = commands
                    .spawn(Window {
                        title: format!("Window #{}", window_count + 1),
                        ..window.clone()
                    })
                    .id();

                let _camera = commands.spawn((
                    Camera {
                        target: RenderTarget::Window(WindowRef::Entity(new_window)),
                        ..Default::default()
                    },
                    Camera3d::default(),
                    Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
                    Msaa::Sample4
                )).insert(FlyCamera {
                    enabled: false,
                    sensitivity: 0.0,
                    ..Default::default()
                });

                return;
            }

            event_writer.write(ev);
        });
    }
}

fn detect_meshes(
    mut state: ResMut<AppState>,
    mesh_entities: Query<&WorldMesh>,
) {
    let mut vertex_count = 0;
    let mut face_count = 0;

    for world_mesh in mesh_entities.iter() {
        vertex_count += world_mesh.vert_count;
        face_count += world_mesh.face_count;
    }

    // Update counts
    state.vert_count = vertex_count;
    state.face_count = face_count;
}

fn setup(
    mut commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    primary_window_query: Single<&mut Window, With<PrimaryWindow>>,
    settings: Res<AppSettings>,
    _state: Res<AppState>,
) {
    // Set primary window to maximized if preferred
    if settings.maximized {
        let mut window = primary_window_query.into_inner();
        window.set_maximized(true);
    }

    // plane
    /*commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.3, 0.5, 0.3),
            double_sided: true,
            unlit: false,
            ..Default::default()
        }),
        ..Default::default()
    });*/

    /*
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(
            shape::Icosphere {
                radius: 0.8,
                subdivisions: 5,
            })
        ),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(1.0, 0.0, 1.0),
            double_sided: true,
            unlit: false,
            ..Default::default()
        }),
        transform: Transform::from_xyz(0.0, 2.0, 0.0),
        ..Default::default()
    });

    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.8, 0.7, 0.6),
            double_sided: true,
            unlit: false,
            ..Default::default()
        }),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..Default::default()
    });*/
    // light
    /*commands.spawn_bundle(LightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });*/
    // camera

    commands.spawn((
        Camera::default(),
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Sample4
    )).insert(FlyCamera {
        enabled: false,
        sensitivity: 0.0,
        ..Default::default()
    });

    // Infinite grid
    commands.spawn(InfiniteGridBundle {
        settings: InfiniteGridSettings {
            fadeout_distance: 300.,
            ..InfiniteGridSettings::default()
        },
        visibility: if settings.show_gridlines {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        ..InfiniteGridBundle::default()
    });
}

fn setup_args(
    _state: ResMut<AppState>,
    mut ev_update_state: EventWriter<AppFileEvent>,
) {
    let mut args = args().skip(1).collect::<Vec<String>>();
    if args.is_empty() {
        return;
    }

    ev_update_state.write(AppFileEvent::Open(args.remove(0).into()));
}

fn consume_file_events(
    mut file_events: EventReader<AppFileEvent>,
    mut app_event_writer: EventWriter<AppEvent>,
    mut state: ResMut<AppState>,
) {
    for e in file_events.read() {
        match e {
            AppFileEvent::Open(file_path) => {
                //milo_event_writer.send(bevy::app::AppExit);
                open_file(file_path, &mut state, &mut app_event_writer);
            }
        }
    }
}

fn consume_app_events(
    mut app_events: EventReader<AppEvent>,
    mut bevy_event_writer: EventWriter<bevy::app::AppExit>,
    mut state: ResMut<AppState>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut textures: ResMut<Assets<Image>>,
    mut grid: Single<&mut Visibility, With<InfiniteGrid>>,
    mut wireframe_config: ResMut<bevy::pbr::wireframe::WireframeConfig>,
    world_meshes: Query<(Entity, &WorldMesh)>,
) {
    for e in app_events.read() {
        match e {
            AppEvent::Exit => {
                bevy_event_writer.write(bevy::app::AppExit::Success);
            },
            AppEvent::SelectMiloEntry(entry_name) => {
                /*let render_entry = match &state.milo_view.selected_entry {
                    Some(name) => name.ne(entry_name),
                    None => true,
                };*/

                // Clear everything
                let mut i = 0;
                for (entity, _) in world_meshes.iter() {
                    i += 1;
                    commands.entity(entity).despawn();
                }
                if i > 0 {
                    debug!("Removed {} meshes in scene", i);
                }

                /*if render_entry {
                    let milo = state.milo.as_ref().unwrap();
                    let info = state.system_info.as_ref().unwrap();

                    render_milo_entry(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &mut textures,
                        milo,
                        Some(entry_name.to_owned()),
                        info
                    );
                }*/

                let milo = state.milo.as_ref().unwrap();
                let milo_path = state.open_file_path.as_ref().unwrap();
                let info = state.system_info.as_ref().unwrap();

                // Render everything for now
                render_milo_entry(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut textures,
                    milo,
                    milo_path,
                    entry_name.to_owned(),
                    info
                );

                state.milo_view.selected_entry = entry_name.to_owned();

                debug!("Updated milo");
            },
            AppEvent::ToggleGridLines(show) => {
                *grid.as_mut() = if *show {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            },
            AppEvent::ToggleWireframes(show) => {
                //grid.single_mut().is_visible = *show;
                wireframe_config.global = *show;
            }
            /*AppEvent::RefreshMilo => {
                return;

                if let Some(milo) = &state.milo {
                    let info = state.system_info.as_ref().unwrap();
                    render_milo(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &mut textures,
                        milo,
                        info
                    );
                }

                debug!("Updated milo");
            },*/
            _ => {
                // Do nothing
            }
        }
    }
}

fn open_file(
    file_path: &PathBuf,
    state: &mut ResMut<AppState>,
    app_event_writer: &mut EventWriter<AppEvent>,
) {
    // Clear file path
    state.open_file_path.take();

    // Get full file extension
    let ext = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| match n.find('.') {
            Some(i) => &n[i..],
            _ => n
        })
        .unwrap();

    // TODO: Make case-insensitive
    if ext.contains("hdr") {
        // Open ark
        info!("Opening hdr from \"{}\"", file_path.display());

        let ark_res = Ark::from_path(file_path);
        if let Ok(ark) = ark_res {
            debug!("Successfully opened ark with {} entries", ark.entries.len());

            state.root = Some(create_ark_tree(&ark));
            state.ark = Some(ark);
            state.open_file_path = Some(file_path.to_owned());
        }
    } else if ext.contains("milo")
        || ext.contains("gh")
        || ext.contains("rnd") { // TODO: Break out into static regex
        // Open milo
        info!("Opening milo from \"{}\"", file_path.display());

        match open_and_unpack_milo(file_path) {
            Ok((milo, info)) => {
                debug!("Successfully opened milo with {} entries", milo.get_entries().len());

                state.milo = Some(milo);
                state.system_info = Some(info);
                state.open_file_path = Some(file_path.to_owned());

                //ev_update_state.send(AppEvent::RefreshMilo);

                const NAME_PREFS: [&str; 5] = ["venue", "top", "lod0", "lod1", "lod2"];

                let _groups = state.milo
                    .as_ref()
                    .unwrap()
                    .get_entries()
                    .iter()
                    .filter(|o| o.get_type() == "Group")
                    .collect::<Vec<_>>();

                let selected_entry = None;
                /*for name in NAME_PREFS {
                    let group = groups
                        .iter()
                        .find(|g| g.get_name().starts_with(name));

                    if let Some(grp) = group {
                        selected_entry = Some(grp.get_name().to_owned());
                        break;
                    }
                }*/

                app_event_writer.write(AppEvent::SelectMiloEntry(selected_entry));
            },
            Err(err) => {
                warn!("Unable to unpack milo file:\n\t: {:?}", err);
            }
        }
    } else {
        info!("Unknown file type \"{}\"", file_path.display());
    }
}

fn create_ark_tree(ark: &Ark) -> ArkDirNode {
    let mut root = ArkDirNode {
        name: ark.path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned(), // There's gotta be a better conversion...
        path: String::from(""),
        dirs: Vec::new(),
        files: Vec::new(),
        loaded: false
    };

    root.expand(ark);
    root
}

fn control_camera(
    key_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut egui_ctx_query: Query<(Entity, &mut EguiContext, &Window, Has<PrimaryWindow>)>,
    mut cam_query: Query<(&Camera, &mut FlyCamera)>,
) {
    let Some((window_entity, mut egui_ctx, window, is_primary)) = egui_ctx_query.iter_mut().find(|(_, _, w, _)| w.focused) else {
        return;
    };

    // egui_ctx_query: Query<(&mut EguiContext, &Window, Has<PrimaryWindow>)>,
    let ctx = egui_ctx.get_mut();

    let key_down = is_camera_button_down(&key_input);
    let mouse_down = mouse_input.pressed(MouseButton::Left);

    for (cam, mut fly_cam) in cam_query.iter_mut() {
        let is_focused = window.focused;
        let cam_matches_window = match (is_primary, &cam.target) {
            (true, RenderTarget::Window(WindowRef::Primary)) => true,
            (false, RenderTarget::Window(WindowRef::Entity(en))) if en.eq(&window_entity) => true,
            _ => false
        };

        // Disable camera move if mouse button not held
        fly_cam.sensitivity = match mouse_down {
            true => 3.0,
            _ => 0.0
        };

        fly_cam.enabled = !ctx.wants_pointer_input()
            && !ctx.is_pointer_over_area()
            && (key_down || mouse_down)
            && (cam_matches_window && is_focused);
    }
}

fn is_camera_button_down(key_input: &Res<ButtonInput<KeyCode>>) -> bool {
    let control_keys = [
        KeyCode::KeyW,
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
        KeyCode::Space,
        KeyCode::ShiftLeft,
    ];

    control_keys
        .iter()
        .any(|k| key_input.pressed(*k))
}

fn window_resized(
    mut resize_events: EventReader<WindowResized>,
    primary_window_query: Single<Entity, With<PrimaryWindow>>,
    mut settings: ResMut<AppSettings>,
    app_state: Res<AppState>,
    winit_windows: NonSend<WinitWindows>,
) {
    let primary_window_id = primary_window_query.into_inner();
    let window = winit_windows.get_window(primary_window_id).unwrap();
    let maximized = window.is_maximized();

    if settings.maximized != maximized {
        if maximized {
            debug!("Window maximized");
        } else {
            debug!("Window unmaximized");
        }

        settings.maximized = maximized;
        app_state.save_settings(&settings);
        return;
    }

    if maximized {
        // Ignore resize if maximized
        return;
    }

    for e in resize_events.read() {
        debug!("Window resized: {}x{}", e.width as u32, e.height as u32);

        settings.window_width = e.width;
        settings.window_height = e.height;
        app_state.save_settings(&settings);
    }
}

fn window_closed(
    _trigger: Trigger<OnDespawn, PrimaryWindow>,
    other_windows_query: Query<(), (With<Window>, Without<PrimaryWindow>)>,
    mut bevy_event_writer: EventWriter<bevy::app::AppExit>,
) {
    let other_windows_open = other_windows_query.iter().any(|_| true);
    if other_windows_open {
        info!("Primary window closed!");
        bevy_event_writer.write(bevy::app::AppExit::Success);
    }
}

fn drop_files(
    mut drag_drop_events: EventReader<FileDragAndDrop>,
    mut file_event_writer: EventWriter<AppFileEvent>,
) {
    for d in drag_drop_events.read() {
        if let FileDragAndDrop::DroppedFile { path_buf, .. } = d {
            debug!("Dropped \"{}\"", path_buf.to_str().unwrap());

            file_event_writer.write(AppFileEvent::Open(path_buf.to_owned()));
        }
    }
}

fn is_drop_event(dad_event: &FileDragAndDrop) -> bool {
    match dad_event {
        FileDragAndDrop::DroppedFile { .. } => true,
        _ => false
    }
}