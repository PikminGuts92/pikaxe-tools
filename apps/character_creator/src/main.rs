// Hide console if release build
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod args;

use args::*;
use bevy::{log::LogPlugin, pbr::wireframe::WireframePlugin, prelude::*, utils::info};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin, InfiniteGridSettings};
use pikaxe::scene::Object;
use pikaxe_bevy::prelude::*;
use std::collections::HashMap;

const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Component)]
pub struct SelectedCharacter;

/*#[derive(Component)]
pub enum HelpText {
    Character(String),

}*/

#[derive(Default, Resource)]
pub struct CharacterAnimations {
    pub enter_clip: Option<Handle<AnimationClip>>,
    pub loop_clip: Option<Handle<AnimationClip>>
}

fn main() {
    let args = CreatorArgs::init();

    App::new()
        .add_plugins((
            DefaultPlugins
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
                }),
            WireframePlugin,
        ))
        /*.add_plugin(bevy::pbr::wireframe::WireframePlugin)
        .insert_resource(bevy::pbr::wireframe::WireframeConfig {
            global: true
        })*/
        .insert_resource(ClearColor(Color::BLACK))
        //.insert_resource(Msaa::Sample4)
        .add_plugins(MiloPlugin {
            ark_path: Some(args.ark_path.into()),
            default_outfit: args.default_outfit,
            ..Default::default()
        })
        .insert_resource(CharacterAnimations::default())
        .add_plugins(FlyCameraPlugin)
        .add_plugins(InfiniteGridPlugin)
        .add_systems(Startup, init_milos)
        .add_systems(Startup, setup)
        .add_systems(Update, fix_meta_proxy_cam)
        .add_systems(Update, control_camera)
        //.add_systems(Update, active_camera_change)
        //.add_system(attach_free_cam)
        .add_systems(Update, load_default_character)
        .add_systems(PostUpdate, set_placer_as_char_parent)
        .add_systems(Update, print_trans_hierarchy)
        .run();
}

// TODO: Move to separate file?
fn init_milos(
    mut scene_events_writer: EventWriter<LoadMiloScene>,
) {
    let default_files = [
        "ui/sel_character.milo",
        "ui/metacam.milo",
        //"world/battle/og/battle_geom.milo",
        //"world/arena/og/arena_geom.milo",
        //"world/fest/og/fest_geom.milo"
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
        mesh: meshes.add(Cuboid::from_size(Vec3::splat(1.0))),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });

    let mut camera = Camera3dBundle::default();
    camera.transform = Transform::from_xyz(-2.0, 2.5, 5.0)
        .looking_at(Vec3::ZERO, Vec3::Y);

    commands
        .spawn(Name::new("Flycam 1"))
        .insert(camera)
        .insert(FlyCamera {
            enabled: false,
            sensitivity: 0.0,
            ..Default::default()
        });

    let mut camera = Camera3dBundle::default();
    camera.camera.is_active = false;
    camera.transform = Transform::from_xyz(-2.0, 2.5, 10.0)
        .looking_at(Vec3::ZERO, Vec3::Y);

    commands
        .spawn(Name::new("Flycam 2"))
        .insert(camera)
        .insert(FlyCamera {
            enabled: false,
            //accel: 400.,
            //max_speed: 5000.,
            sensitivity: 0.0,
            ..Default::default()
        }); // Fix camera

    // Infinite grid
    commands.spawn(InfiniteGridBundle {
        settings: InfiniteGridSettings {
            fadeout_distance: 300.,
            ..InfiniteGridSettings::default()
        },
        visibility: Visibility::Hidden,
        ..InfiniteGridBundle::default()
    });

    /*commands
        .spawn(
            TextBundle::from_section(
                "Character: Judy Nails",
                TextStyle::default())
                .with_text_alignment(TextAlignment::Left)
                .with_style(Style {
                    position_type: PositionType::Relative,
                    //left: Val::Px(5.),
                    //top: Val::Percent(5.),
                    ..Default::default()
                }));*/

    /*commands.spawn_batch([
        HelpText()
    ]);*/
}

fn control_camera(
    key_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut cam_query: Query<(&mut Camera, Option<&mut FlyCamera>, Option<&Name>)>,
) {
    let key_down = is_camera_button_down(&key_input);
    let mouse_down = mouse_input.pressed(MouseButton::Left);

    let count = cam_query.iter().count();
    let cycle_cam = key_input.any_just_released([KeyCode::KeyC]);

    let current_idx = cam_query
        .iter()
        .enumerate()
        .find(|(_, (c, ..))| c.is_active)
        .map(|(i, _)| i)
        .unwrap_or_default();

    let next_idx = match (cycle_cam, current_idx + 1) {
        (true, next) if next < count => next,
        (true, _) => 0,
        _ => current_idx
    };

    for (i, (mut cam, fly_cam, name)) in cam_query.iter_mut().enumerate() {
        let was_active = cam.is_active;
        cam.is_active = i == next_idx;

        if was_active != cam.is_active {
            log::debug!("Cam {} is now active", name.as_ref().map(|n| n.as_str()).unwrap_or("(unknown)"));
        }

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

/*fn active_camera_change(
   cam_query: Query<(&Camera, Option<&Name>), Changed<Camera>>,
) {
    for (cam, name) in cam_query.iter() {
        if cam.is_active {
            log::debug!("Cam {} is now active", name.as_ref().map(|n| n.as_str()).unwrap_or("(unknown)"));
        }
    }
}*/

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

fn fix_meta_proxy_cam(
    mut cam_query: Query<(&mut Transform, &Name), (Added<Transform>, With<Camera>)>,
) {
    const CAM_NAME: &str = "meta.cam";

    for (mut trans, name) in cam_query.iter_mut() {
        if name.as_str() == CAM_NAME {
            *trans = trans.looking_at(Vec3::ZERO, Vec3::Z);
            log::error!("Patched transform in {CAM_NAME}");
        }
    }
}

fn print_trans_hierarchy(
    key_input: Res<ButtonInput<KeyCode>>,
    root_query: Query<Entity, With<MiloRoot>>,
    milo_query: Query<Entity, With<MiloObject>>,
    trans_query: Query<(Entity, Option<&Name>, Option<&Children>), With<Transform>>,
) {
    let show_hierarchy = key_input.any_just_released([KeyCode::KeyH]);

    if !show_hierarchy {
        return;
    }

    let milo_count = milo_query.iter().count();
    println!("Found {milo_count} milos");

    let root_entity = root_query.single();
    let trans_map = trans_query
        .iter()
        .map(|(e, n, c)| (e, (n, c)))
        .collect::<HashMap<_, _>>();

    print_children(&root_entity, &trans_map, 0);
}

fn print_children(
    parent_entity: &Entity,
    trans_map: &HashMap<Entity, (Option<&Name>, Option<&Children>)>,
    index: usize,
) {
    let Some((parent_name, children)) = trans_map.get(parent_entity) else {
        return;
    };

    println!(
        "{}{}",
        "  ".repeat(index),
        parent_name.map(|pn| pn.as_str()).unwrap_or("(unknown)")
    );

    let Some(children) = children else {
        return;
    };

    for child_entity in children.iter() {
        print_children(child_entity, trans_map, index + 1);
    }
}

/*fn attach_free_cam(
    mut commands: Commands,
    cam_query: Query<Entity, (&Camera, Without<FlyCamera>)>,
) {
    for cam_entity in cam_query.iter() {
        commands
            .entity(cam_entity)
            .insert(FlyCamera {
                enabled: false,
                //accel: 10.,
                //max_speed: 50.,
                sensitivity: 0.0,
                ..Default::default()
            });
    }
}*/

fn load_default_character(
    mut commands: Commands,
    mut scene_events_writer: EventWriter<LoadMiloSceneWithCommands>,
    mut animations: ResMut<Assets<AnimationClip>>,
    placer_query: Query<(Entity, &Name), Added<MiloBandPlacer>>,
    state: Res<MiloState>,
) {
    let Ok((placer_entity, placer_name)) = placer_query.get_single() else {
        return
    };

    // Load character
    scene_events_writer.send(
        LoadMiloSceneWithCommands(
            "char/alterna1/og/alterna1_ui.milo".into(),
            //"char/grim/og/grim_ui.milo".into(),
            |commands| {
                commands.insert(SelectedCharacter);
            }
        )
    );

    /*let Some((_info, anims_obj_dir)) = state.open_milo("char/alterna1/anims/alterna1_ui.milo") else {
        return
    };

    let samples = anims_obj_dir
        .get_entries()
        .iter()
        .filter_map(|e| match e {
            Object::CharClipSamples(ccs) => Some(ccs),
            _ => None
        })
        .collect::<Vec<_>>();

    let loop_sample = samples
        .iter()
        .find(|ccs| ccs.name == "ui_loop")
        .unwrap();*/

    // Setup animation on placer
    let mut anim_player = AnimationPlayer::default();
    let mut anim_clip = AnimationClip::default();

    anim_clip
        .add_curve_to_path(
            EntityPath {
                parts: vec![placer_name.to_owned()]
            },
            VariableCurve {
                keyframe_timestamps: vec![0.0, 1.0, 2.0, 3.0, 4.0],
                keyframes: Keyframes::Rotation(vec![
                    Quat::IDENTITY,
                    Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.),
                    Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2. * 2.),
                    Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2. * 3.),
                    Quat::IDENTITY,
                ]),
                interpolation: Interpolation::Linear,
            },
        );

    anim_player
        .play(animations.add(anim_clip))
        .repeat();

    commands
        .entity(placer_entity)
        .insert(anim_player);
}

fn set_placer_as_char_parent(
    mut commands: Commands,
    //mut scene_events_reader: EventReader<LoadMiloSceneComplete>,
    state: Res<MiloState>,
    char_objects_query: Query<(Entity, Option<&Parent>, &MiloObject), Added<SelectedCharacter>>,
    placer_query: Query<Entity, With<MiloBandPlacer>>,
) {
    let Ok(placer_entity) = placer_query.get_single() else {
        return
    };

    if char_objects_query.is_empty() {
        return
    }

    // TODO: Remove when Character entry can be parsed
    /*let root_entity = root_query.single();
    let mat = Mat4::IDENTITY;

    let trans_entity = commands
        .spawn(Name::new("alterna1"))
        .insert(SpatialBundle {
            transform: Transform::from_matrix(mat),
            ..Default::default()
        })
        .insert(MiloObject {
            id: (start_idx + i) as u32,
            name: String::from("alterna1"),
            dir: String::from("alterna1"),
        })
        .id();

    commands
        .entity(root_entity)
        .add_child(trans_entity);

    update_parents_events_writer.send(UpdateMiloObjectParents);*/

    //return;

    info!("Updating parents for band placer");

    for (entity, parent, obj) in char_objects_query.iter() {
        let obj_dir_name = obj.dir.as_str();

        let Some(obj) = state.objects.get(obj.id as usize) else {
            continue;
        };

        let trans_parent = match obj {
            Object::BandPlacer(obj) => &obj.parent,
            Object::Cam(obj) => &obj.parent,
            Object::Mesh(obj) => &obj.parent,
            Object::Group(obj) => &obj.parent,
            Object::Trans(obj) => &obj.parent,
            _ => todo!("Shouldn't happen")
        };

        // Add root-level trans objects to placer
        if trans_parent.is_empty() || trans_parent.eq(obj_dir_name) {
            // Manually update old parent
            /*if let Some(parent) = parent {
                error!("Updating old parent");

                commands
                    .entity(parent.get())
                    .remove_children(&[entity]);

                commands
                    .entity(entity)
                    .remove_parent();

                commands
                    .entity(entity)
                    .remove::<Parent>();
            }*/

            commands
                .entity(placer_entity)
                .add_child(entity);

            /*commands
                .entity(entity)
                .set_parent(placer_entity);*/

            /*commands
                .entity(entity)
                .add(Parent:: Parent(placer_entity));*/
        }
    }
}