// Hide console if release build
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod args;
mod gui;
mod resources;

use args::*;
use gui::GuiPlugin;
use resources::*;

use bevy::{animation::{animated_field, AnimationTarget, AnimationTargetId}, input::common_conditions::input_just_pressed, log::{info, LogPlugin}, pbr::wireframe::WireframePlugin, prelude::*};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin, InfiniteGridSettings};
use bevy_rapier3d::prelude::*;
use pikaxe::scene::Object;
use pikaxe_bevy::prelude::*;
use std::collections::HashMap;

const _PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Component)]
pub struct SelectedCharacterComponent; // TODO: Rename to something else (same for selected anim component)

#[derive(Component)]
pub struct SelectedAnimationComponent;

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
            WireframePlugin::default(),
        ))
        /*.add_plugin(bevy::pbr::wireframe::WireframePlugin)
        .insert_resource(bevy::pbr::wireframe::WireframeConfig {
            global: true
        })*/
        .insert_resource(ClearColor(Color::BLACK))
        //.insert_resource(Msaa::Sample4)

        // Shared resources
        .insert_resource(SelectedCharacter::default())
        .insert_resource(SelectedCharacterOptions::default())
        .insert_resource(SelectedAnimation::default())
        .insert_resource(SelectedAnimationOptions::default())

        .add_plugins(MiloPlugin {
            ark_path: Some(args.ark_path.into()),
            default_outfit: args.default_outfit,
            ..Default::default()
        })
        .add_plugins(GuiPlugin)
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
        /*.add_systems(Update, (
            show_debug_gizmos_for_bones,
            show_debug_gizmos_for_char_hair
        ))*/
        .add_systems(Update, toggle_char_mesh_visibility.run_if(input_just_pressed(KeyCode::KeyM)))
        .add_systems(Update, toggle_play_anims.run_if(input_just_pressed(KeyCode::KeyP)))
        .add_systems(PostUpdate, (set_placer_as_char_parent, play_anim_after_load))
        .add_systems(Update, print_trans_hierarchy)

        // Update char
        .add_systems(Update, change_character.run_if(resource_changed::<SelectedCharacter>))

        // Update anim
        .add_systems(Update, change_animation.run_if(resource_changed::<SelectedAnimation>))

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
        scene_events_writer.write(LoadMiloScene(milo_path.to_string()));
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    /*commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(1.0)))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));*/

    commands
        .spawn(Name::new("Flycam 1"))
        .insert((
            Camera3d::default(),
            Camera::default(), // TODO: Maybe use 'target' instead of transform
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y)
        ))
        .insert(FlyCamera {
            enabled: false,
            sensitivity: 0.0,
            ..Default::default()
        });

    commands
        .spawn(Name::new("Flycam 2"))
        .insert((
            Camera3d::default(),
            Camera {
                is_active: false, // TODO: Maybe use 'target' instead of transform
                ..Default::default()
            }, 
            Transform::from_xyz(-2.0, 2.5, 10.0).looking_at(Vec3::ZERO, Vec3::Y)
        ))
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
    trans_query: Query<(Entity, Option<&Name>, Option<&Children>)>,
) {
    let show_hierarchy = key_input.any_just_released([KeyCode::KeyH]);

    if !show_hierarchy {
        return;
    }

    let milo_count = milo_query.iter().count();
    println!("Found {milo_count} milos");

    let root_entity = root_query.single().unwrap(); // TODO: Better handle unwrap
    let trans_map = trans_query
        .iter()
        .map(|(e, n, c)| (e, (n, c)))
        .collect::<HashMap<_, _>>();

    print_children(root_entity, &trans_map, 0);
}

fn print_children(
    parent_entity: Entity,
    trans_map: &HashMap<Entity, (Option<&Name>, Option<&Children>)>,
    index: usize,
) {
    let Some((parent_name, children)) = trans_map.get(&parent_entity) else {
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

fn change_character(
    mut commands: Commands,
    selected_character: Res<SelectedCharacter>,
    selcted_character_options: Res<SelectedCharacterOptions>,
    selected_character_query: Query<Entity, With<SelectedCharacterComponent>>,
    mut scene_events_writer: EventWriter<LoadMiloSceneWithCommands>,
) {
    // Clear existing character
    for char_entity in selected_character_query.iter() {
        commands
            .entity(char_entity)
            .despawn();
    }

    let Some((shortname, _, is_guitarist)) = selected_character.0.and_then(|s| selcted_character_options.0.get(s)) else {
        return;
    };

    // Load character
    scene_events_writer.write(
        LoadMiloSceneWithCommands(
            format!("char/{shortname}/og/{shortname}{}.milo", if *is_guitarist { "_ui" } else { "" }),
            |commands| {
                commands.insert(SelectedCharacterComponent);
            }
        )
    );
}

fn change_animation(
    mut commands: Commands,
    selected_animation: Res<SelectedAnimation>,
    selcted_animation_options: Res<SelectedAnimationOptions>,
    selected_animation_query: Query<Entity, With<SelectedAnimationComponent>>,
    current_anim_targets_query: Query<(Entity, Option<&OriginalTransform>), With<AnimationTarget>>,
    mut scene_events_writer: EventWriter<LoadMiloSceneWithCommands>,
) {
    // Clear existing animation
    for anim_entity in selected_animation_query.iter() {
        commands
            .entity(anim_entity)
            .despawn();
    }

    // Clean anim targets
    for (anim_target_entity, orig_trans) in current_anim_targets_query.iter() {
        commands
            .entity(anim_target_entity)
            .remove::<AnimationTarget>();

        // Reset local transform
        if let Some(&OriginalTransform(trans)) = orig_trans {
            commands
                .entity(anim_target_entity)
                .insert(trans);
        }
    }

    let Some((shortname, _)) = selected_animation.0.and_then(|s| selcted_animation_options.0.get(s)) else {
        return;
    };

    // Load animation
    scene_events_writer.write(
        LoadMiloSceneWithCommands(
            format!("char/{shortname}/anims/{shortname}_ui.milo"),
            |commands| {
                commands.insert(SelectedAnimationComponent);
            }
        )
    );
}

fn load_default_character(
    mut commands: Commands,
    mut scene_events_writer: EventWriter<LoadMiloSceneWithCommands>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut selected_character: ResMut<SelectedCharacter>,
    placer_query: Query<(Entity, &Name), Added<MiloBandPlacer>>,
    root_query: Single<Entity, With<MiloRoot>>,
    _state: Res<MiloState>,
) {
    let Ok((placer_entity, placer_name)) = placer_query.single() else {
        return
    };

    let root_entity = root_query.into_inner();

    selected_character.0 = Some(12); // Judy Nails

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

    return;

    // Setup animation on placer
    let mut anim_player = AnimationPlayer::default();
    let mut anim_clip = AnimationClip::default();

    // TODO: Get anim target id more directly?
    let anim_target_id = AnimationTargetId::from_name(placer_name);

    anim_clip
        .add_curve_to_target(
            anim_target_id,
            /*VariableCurve {
                keyframe_timestamps: vec![0.0, 1.0, 2.0, 3.0, 4.0],
                keyframes: Keyframes::Rotation(vec![
                    Quat::IDENTITY,
                    Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.),
                    Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2. * 2.),
                    Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2. * 3.),
                    Quat::IDENTITY,
                ]),
                interpolation: Interpolation::Linear,
            },*/
            AnimatableCurve::new(
                animated_field!(Transform::rotation),
                AnimatableKeyframeCurve::new([0.0, 1.0, 2.0, 3.0, 4.0].into_iter().zip([
                    Quat::IDENTITY,
                    Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.),
                    Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2. * 2.),
                    Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2. * 3.),
                    Quat::IDENTITY,
                ]))
                .expect("Failed to build rotation curve")
            )
        );

    // TODO: Better keep track of anim clip
    //let anim_clip = animations.add(anim_clip);
    //let mut anim_graph = AnimationGraph::new();
    //let node_idx = anim_graph.add_clip(anim_clip, 1.0, anim_graph.root);
    let (anim_graph, node_idx) = AnimationGraph::from_clip(animations.add(anim_clip));

    anim_player
        .play(node_idx)
        .repeat();

    commands
        .entity(placer_entity)
        .insert(AnimationTarget {
            id: anim_target_id,
            player: root_entity,
        });

    commands
        .entity(root_entity)
        .insert((
            anim_player,
            AnimationGraphHandle(animation_graphs.add(anim_graph))
        ));
}

fn set_placer_as_char_parent(
    mut commands: Commands,
    mut scene_events_writer: EventWriter<LoadMiloSceneWithCommands>,
    //mut scene_events_reader: EventReader<LoadMiloSceneComplete>,
    mut selected_animation: ResMut<SelectedAnimation>,
    state: Res<MiloState>,
    char_objects_query: Query<(Entity, Option<&ChildOf>, &MiloObject), (Added<SelectedCharacterComponent>, Without<CustomParent>, Without<ParentOverride>)>,
    placer_query: Query<Entity, With<MiloBandPlacer>>,
    root_anim_query: Option<Single<(), (With<MiloRoot>, With<AnimationPlayer>)>>,
) {
    let Ok(placer_entity) = placer_query.single() else {
        return
    };

    if char_objects_query.is_empty() {
        return
    }

    // Load anim
    if root_anim_query.is_none() {
        // Only reload animation if not already set
        selected_animation.0 = Some(7); // Judy Nails
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

    for (entity, _parent, obj) in char_objects_query.iter() {
        let obj_dir_name = obj.dir.as_str();

        let Some(obj) = state.objects.get(&obj.id) else {
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

            log::debug!("Adding {} to placer entity", obj.get_name());

            commands
                .entity(placer_entity)
                .add_child(entity);

            commands
                .entity(entity)
                .insert(ParentOverride);

            /*commands
                .entity(entity)
                .set_parent(placer_entity);*/

            /*commands
                .entity(entity)
                .add(Parent:: Parent(placer_entity));*/
        }
    }
}

fn play_anim_after_load(
    mut commands: Commands,
    state: Res<MiloState>,
    mut char_anims: ResMut<CharacterAnimations>,
    milo_anims_query: Query<(Entity, &MiloCharClip, &MiloObject), Added<SelectedAnimationComponent>>,
    trans_query: Query<(Entity, &Name), (With<MiloObject>, With<Transform>)>,
    root_query: Single<Entity, With<MiloRoot>>,
    animations: Res<Assets<AnimationClip>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
) {
    let play_clip = !milo_anims_query.is_empty();

    for (_entity, MiloCharClip(anim_clip_handle), milo_object) in milo_anims_query.iter() {
        match milo_object.name.as_str() {
            "ui_enter" => {
                char_anims.enter_clip = Some(anim_clip_handle.clone());
            },
            "ui_loop" => {
                char_anims.loop_clip = Some(anim_clip_handle.clone());
            },
            _ => {}
        }
    }

    // TODO: Move to event/observer or different system?
    if !play_clip || (char_anims.enter_clip.is_none() && char_anims.loop_clip.is_none()) {
        return;
    }

    let root_entity = root_query.into_inner();

    let mut anim_player = AnimationPlayer::default();
    //let mut anim_graph = AnimationGraph::new();
    //let node_idx = anim_graph.add_clip(anim_clip, 1.0, anim_graph.root);

    // TODO: Combine enter + loop clips somehow
    //let anim_enter_handle = char_anims.enter_clip.as_ref().unwrap();
    let anim_loop_handle = char_anims.loop_clip.as_ref().unwrap();

    let (anim_graph, loop_node_index) = AnimationGraph::from_clip(anim_loop_handle.clone());
    //let loop_node_index = anim_graph.add_clip(anim_loop_handle.clone(), 1.0, anim_graph.root);

    let anim_loop = animations.get(anim_loop_handle).unwrap();

    for (entity, name) in trans_query.iter() {
        let trans_target_id = AnimationTargetId::from_name(name);
        if anim_loop.curves().contains_key(&trans_target_id) {
            commands
                .entity(entity)
                .insert(AnimationTarget {
                    id: trans_target_id,
                    player: root_entity,
                });
        }
    }

    /*let end_anim_clip = {
        let mut clip = AnimationClip::default();
        clip.add_event(anim_enter.duration(), PlayClip(anim_loop_handle.clone()));
        clip
    };
    let node_idx = anim_graph.add_clip(animations.add(end_anim_clip), 1.0, anim_graph.root);*/

    anim_player
        .play(loop_node_index)
        .set_speed(30.0)
        .repeat();

    commands
        .entity(root_entity)
        .insert((
            anim_player,
            AnimationGraphHandle(animation_graphs.add(anim_graph))
        ));

    log::debug!("Playing character animation!");
}

fn update_anim_targets(
    //query: Query<SelectedCharacterComponent>,
    trans_query: Query<(Entity, &Name), (With<MiloObject>, With<Transform>)>,
) {
    // Check if either selected animation changed or selected character changed
}

fn show_debug_gizmos_for_bones(
    mut gizmos: Gizmos,
    bones_query: Query<(Entity, &GlobalTransform, Option<&ChildOf>), (With<MiloBone>, With<SelectedCharacterComponent>)>,
) {
    use bevy::color::palettes::css::*;

    //return;

    for (_, trans, parent) in bones_query.iter() {
        let pos = trans.translation();
        //let next_pos = trans.mul_transform(Transform::from_translation(Vec3::splat(5.0))).translation();

        gizmos.sphere(pos, 1.0, LIMEGREEN);

        //gizmos.arrow(pos, next_pos, BLUE);
        //gizmos.axes(*trans, 1.0);

        let Some(ChildOf(parent)) = parent else {
            continue;
        };

        let Ok((_, parent_trans, _)) = bones_query.get(*parent) else {
            continue;
        };

        gizmos.line(parent_trans.translation(), pos, BLUE);
    }
}

fn show_debug_gizmos_for_char_hair(
    mut gizmos: Gizmos,
    hair_strands_query: Query<(&Name, &GlobalTransform, Option<&ChildOf>), (With<MiloCharHair>, With<SelectedCharacterComponent>)>,
) {
    use bevy::color::palettes::css::*;

    for (name, trans, parent) in hair_strands_query.iter() {
        //log::debug!("Strand: {}", name.as_str());

        let pos = trans.translation();
        //let next_pos = trans.mul_transform(Transform::from_translation(Vec3::splat(5.0))).translation();

        let is_parent_hair = parent
            .map(|&ChildOf(p)| hair_strands_query.contains(p))
            .unwrap_or_default();

        //gizmos.sphere(pos, 1.0, if is_parent_hair { RED } else { ORANGE });

        //gizmos.arrow(pos, next_pos, BLUE);
        //gizmos.axes(*trans, 1.0);

        let Some(ChildOf(parent)) = parent else {
            continue;
        };

        let Ok((_, parent_trans, _)) = hair_strands_query.get(*parent) else {
            continue;
        };

        gizmos.line(parent_trans.translation(), pos, PURPLE);
    }
}

fn toggle_char_mesh_visibility(
    mut hide_meshes: Local<bool>,
    mut char_mesh_query: Query<&mut Visibility, (With<MiloMesh>, With<SelectedCharacterComponent>)>,
) {
    *hide_meshes = !*hide_meshes;

    for mut mesh_visibility in char_mesh_query.iter_mut() {
        *mesh_visibility = match *hide_meshes {
            true => Visibility::Hidden,
            _ => Visibility::Inherited,
        }
    }

    log::info!("Toggle hide meshes: {}", if *hide_meshes { "hidden" } else { "visible" });
}

fn toggle_play_anims(
    mut anims_paused: Local<bool>,
    mut commands: Commands,
    mut anim_player_query: Query<&mut AnimationPlayer>,
    rigid_body_query: Query<Entity, With<RigidBody>>,
) {
    for mut anim_player in anim_player_query.iter_mut() {
        if *anims_paused {
            anim_player.resume_all();
            for rigid_body_entity in rigid_body_query.iter() {
                commands
                    .entity(rigid_body_entity)
                    .remove::<RigidBodyDisabled>();
            }
        } else {
            anim_player.pause_all();
            for rigid_body_entity in rigid_body_query.iter() {
                commands
                    .entity(rigid_body_entity)
                    .insert(RigidBodyDisabled)
                    .insert(Velocity::zero());
            }
        }

        *anims_paused = !*anims_paused;
        log::info!("Toggle animations: {}", if *anims_paused { "paused" } else { "playing" });
    }
}