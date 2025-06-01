use crate::prelude::*;
use bevy::animation::{animated_field, AnimationTarget, AnimationTargetId};
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::AsyncComputeTaskPool;
use bevy_mod_inverse_kinematics::*;
use bevy_rapier3d::prelude::*;
use std::collections::{HashMap, HashSet};
use futures_lite::future;
use pikaxe::ark::Ark;
use pikaxe::scene::{Blend, EncodedSamples, Matrix, Matrix3, MiloObject as MObject, Object, ObjectDir, ObjectDirBase, RndMesh, Sphere as MiloSphere, Trans, ZMode};
use pikaxe::texture::Bitmap;
use pikaxe::{Platform, SystemInfo};
use std::path::PathBuf;

const MILO_TO_BEVY_MATRIX: Mat4 = Mat4::from_cols_array(&[
    -1.0,  0.0,  0.0, 0.0,
    0.0,  0.0,  1.0, 0.0,
    0.0,  1.0,  0.0, 0.0,
    0.0,  0.0,  0.0, 1.0,
]);

#[derive(Default)]
pub struct MiloPlugin {
    pub ark_path: Option<PathBuf>,
    pub default_outfit: Option<String>,
}

enum LoadMilo {
    FromArkPath(String),
    FromObjects(Vec<Object>)
}

impl Plugin for MiloPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RapierPhysicsPlugin::<NoUserData>::default(),
            RapierDebugRenderPlugin::default(),
            InverseKinematicsPlugin
        ));

        // Open ark
        let state = MiloState {
            ark: self.ark_path
                .as_ref()
                .map(|p| Ark::from_path(p).expect("Can't open ark file")),
            ..Default::default()
        };

        app.add_event::<ClearMiloScene>();
        app.add_event::<LoadMiloScene>();
        app.add_event::<LoadMiloSceneWithCommands>();
        app.add_event::<LoadMiloObjectsWithCommands>();
        app.add_event::<LoadMiloSceneComplete>();
        app.add_event::<UpdateSkinnedMeshes>();

        app.insert_resource(state);
        app.insert_resource(MiloEntityMap::default());

        app.add_systems(Startup, init_world);

        app.add_systems(Update, (
            consolidate_milo_scene_events.pipe(process_milo_scene_events).pipe(update_milo_object_parents),
            update_skinned_meshes,
            update_skinned_meshes_ik
        ).chain());

        app.add_systems(PostUpdate, (
            (clone_transforms, clone_world_transforms).after(TransformSystem::TransformPropagate),
            clone_world_transforms_with_physics.after(PhysicsSet::Writeback)
        ));

        app.add_systems(Update, process_milo_async_textures);
        app.add_observer(add_milo_object);
        app.add_observer(remove_milo_object);

        app.add_observer(set_clone_world_transform);
    }
}

fn add_milo_object(
    trigger: Trigger<OnInsert, MiloObject>,
    milo_object_query: Query<&MiloObject>,
    mut milo_entity_map: ResMut<MiloEntityMap>,
) {
    let milo_object = milo_object_query.get(trigger.target()).unwrap(); // Observers with results not supported
    milo_entity_map.set_entity(&milo_object.name, trigger.target());
}

fn remove_milo_object(
    trigger: Trigger<OnRemove, MiloObject>,
    milo_object_query: Query<&MiloObject>,
    mut milo_entity_map: ResMut<MiloEntityMap>,
) {
    let milo_object = milo_object_query.get(trigger.target()).unwrap(); // Observers with results not supported
    milo_entity_map.remove(&milo_object.name);
}

fn set_clone_world_transform(
    trigger: Trigger<SetCloneWorldTransform>,
    mut commands: Commands,
    milo_entity_map: Res<MiloEntityMap>,
) {
    let cloned_entity = trigger.0;
    let entity = milo_entity_map.get_entity(&trigger.1).unwrap();

    commands
        .entity(entity)
        .remove::<ChildOf>()
        //.remove::<GlobalTransform>()
        //.insert(Transform::default())
        .insert((
            ParentOverride,
            PhysicsControlledBone,
            CloneWorldTransform(cloned_entity)
        ));
}


fn init_world(
    mut commands: Commands,
) {
    /*
    // TODO: Move to exe project?
    // Ground
    let ground_size = 1000.1;
    let ground_height = 0.1;
    commands.spawn((
        Transform::from_xyz(0.0, -ground_height, 0.0),
        Collider::cuboid(ground_size, ground_height, ground_size),
    ));

    let new_parent = commands
        .spawn_empty()
        .insert((
            Transform::from_xyz(0., 30., 0.),
        ))
        .id();

    let ball_parent = commands
        .spawn_empty()
        .insert(RigidBody::Fixed)
        //.insert(Collider::ball(2.0))
        //.insert(ChildOf(new_parent))
        .insert(Transform::from_xyz(0., 35., 0.))
        .id();

    commands
        .spawn(RigidBody::Dynamic)
        .insert(Collider::ball(0.5))
        .insert(Restitution::coefficient(0.7))
        .insert(Transform::from_xyz(0.0, 4.0, 0.0));

    let cube_size = 0.1;
    let joint = RopeJointBuilder::new(6.0);
    let parent = commands
        .spawn(RigidBody::Dynamic)
        .insert(Collider::cuboid(cube_size, cube_size, cube_size))
        //.insert(Transform::from_xyz(0.0, 0.0, 1.0))
        //.insert(ChildOf(ball_parent))
        .insert(ImpulseJoint::new(ball_parent, joint))
        .id();
    let parent = commands
        .spawn(RigidBody::Dynamic)
        .insert(Collider::cuboid(cube_size, cube_size, cube_size))
        //.insert(Transform::from_xyz(0.0, 0.0, 1.0))
        //.insert(ChildOf(parent))
        .insert(ImpulseJoint::new(parent, joint))
        .id();
    let parent = commands
        .spawn(RigidBody::Dynamic)
        //.insert(LockedAxes::TRANSLATION_LOCKED)
        .insert(Collider::cuboid(cube_size, cube_size, cube_size))
        //.insert(ColliderMassProperties::Mass(20.))
        //.insert(Transform::from_xyz(0.0, 0.0, 1.0))
        //.insert(ChildOf(parent))
        .insert(ImpulseJoint::new(parent, joint))
        .id();

    // Translate to bevy coordinate system
    //let scale_mat = Mat4::from_scale(Vec3::new(0.1, 0.1, 0.1));*/
    let trans = Transform::from_matrix(MILO_TO_BEVY_MATRIX);

    commands
        .spawn(Name::new("Root"))
        .insert((
            trans,
            GlobalTransform::from(trans), // TODO: Possibly remove
            Visibility::Inherited
        ))
        .insert(MiloRoot);
}

fn consolidate_milo_scene_events(
    mut scene_events: ResMut<Events<LoadMiloScene>>,
    mut scene_events_commands:  ResMut<Events<LoadMiloSceneWithCommands>>,
    mut objects_events_commands:  ResMut<Events<LoadMiloObjectsWithCommands>>,
) -> Vec<(LoadMilo, Option<fn(&mut EntityCommands)>)> {
    return scene_events
        .drain()
        .map(|LoadMiloScene(p)| (LoadMilo::FromArkPath(p), None))
        .chain(scene_events_commands
            .drain()
            .map(|LoadMiloSceneWithCommands(p, c)| (LoadMilo::FromArkPath(p), Some(c)))
        )
        .chain(objects_events_commands
            .drain()
            .map(|LoadMiloObjectsWithCommands(o, c)| (LoadMilo::FromObjects(o), Some(c)))
        )
        .collect()
}

// TODO: Move to separate file?
fn process_milo_scene_events(
    load_milo_events: In<Vec<(LoadMilo, Option<fn(&mut EntityCommands)>)>>,
    mut commands: Commands,
    mut state: ResMut<MiloState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut scene_events_writer: EventWriter<LoadMiloSceneComplete>,
    mut update_skinned_meshes_events_writer: EventWriter<UpdateSkinnedMeshes>,
    root_query: Query<Entity, With<MiloRoot>>,
) -> bool {
    /*for e in state.ark.as_ref().unwrap().entries.iter() {
        log::debug!("{}", &e.path);
    }*/

    let thread_pool = AsyncComputeTaskPool::get();
    let root_entity = root_query.single().unwrap(); // TODO: Handle safely

    let mut milos_updated = false;

    /*for scene in scene_events_reader.in {

    }*/

    // TODO: Check if path ends in .milo
    for (load_milo, callback) in load_milo_events.0 {
        let (sys_info, mut milo) = match load_milo {
            LoadMilo::FromArkPath(milo_path) => {
                log::debug!("Loading Scene: \"{}\"", milo_path);
                state.open_milo(&milo_path).unwrap()
            },
            LoadMilo::FromObjects(milo_objects) => {
                let sys_info = SystemInfo::default();
                let milo_obj = ObjectDir::ObjectDir(ObjectDirBase {
                    entries: milo_objects,
                    name: String::from("custom"),
                    dir_type: String::from("Object"),
                    sub_dirs: Vec::new(),
                });

                (sys_info, milo_obj)
            }
        };

        let (obj_dir_name, obj_dir_type) = match &milo {
            ObjectDir::ObjectDir(dir) => (&dir.name, &dir.dir_type)
        };

        //let mut texture_map = HashMap::new(); // name -> tex future

        let (milo_textures, milo_materials, milo_meshes) = milo
            .get_entries()
            .iter()
            .fold((HashMap::new(), HashMap::new(), HashMap::new()), |(mut tx, mut mt, mut ms), e| {
                match e {
                    Object::Mat(mat) => {
                        mt.insert(mat.get_name(), mat);
                    },
                    Object::Mesh(mesh) => {
                        ms.insert(mesh.get_name(), mesh);
                    },
                    Object::Tex(tex) => {
                        tx.insert(tex.get_name(), tex);
                    },
                    _ => {}
                }

                (tx, mt, ms)
            });

        // Mesh -> mat
        let materials_to_load = milo_meshes
            .values()
            .map(|m| &m.mat)
            .collect::<HashSet<_>>()
            .iter()
            .flat_map(|m| milo_materials.get(m).as_ref().map(|m| *m))
            .collect::<Vec<_>>();

        // Mesh -> mat -> tex (create tasks)
        let mut textures_to_load = materials_to_load
            .iter()
            .fold(HashSet::new(), |mut acc, m| {
                acc.insert(&m.diffuse_tex);
                acc.insert(&m.normal_map);
                acc.insert(&m.emissive_map);

                acc
            })
            .into_iter()
            .flat_map(|t| milo_textures.get(t).map(|t| *t))
            .filter(|t| t.bitmap.is_some())
            .map(|tex| {
                let sys_info = sys_info.clone();

                /*let name = tex
                    .get_name()
                    .to_owned();*/

                let bitmap = tex.bitmap
                    .as_ref()
                    .unwrap()
                    .clone();

                let task = thread_pool.spawn(async move {
                    // Decode texture
                    let (decoded, bpp, format) = match (&sys_info.platform, bitmap.encoding) {
                        (Platform::X360 | Platform::PS3, enc @ (8 | 24 | 32)) => {
                            let mut data = bitmap.raw_data;

                            if sys_info.platform.eq(&Platform::X360) {
                                // Swap bytes
                                for ab in data.chunks_mut(2) {
                                    let tmp = ab[0];

                                    ab[0] = ab[1];
                                    ab[1] = tmp;
                                }
                            }

                            let format = match enc {
                                24 => TextureFormat::Bc3RgbaUnormSrgb, // DXT5
                                32 => TextureFormat::Bc5RgUnorm,       // ATI2
                                _  => TextureFormat::Bc1RgbaUnormSrgb, // DXT1
                            };

                            (data, bitmap.bpp as usize, format)
                        },
                        _ => {
                            let data = bitmap.unpack_rgba(&sys_info)
                                .expect("Can't decode \"{name}\" texture");

                            (data, 32, TextureFormat::Rgba8UnormSrgb)
                        }
                    };

                    let Bitmap { width, height, mip_maps, .. } = bitmap;

                    let tex_size = ((width as usize) * (height as usize) * bpp) / 8;
                    let use_mips = false; // TODO: Always support mips?

                    let img_slice = if use_mips {
                        &decoded
                    } else {
                        &decoded[..tex_size]
                    };

                    let image_new_fn = match format {
                        TextureFormat::Rgba8UnormSrgb => image_new_fill, // Use fill method for older textures
                        _ => image_new,
                    };

                    let mut texture = /*Image::new_fill*/ image_new_fn(
                        Extent3d {
                            width: width.into(),
                            height: height.into(),
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        img_slice,
                        format
                    );

                    // Update texture wrap mode
                    texture.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::Repeat,
                        address_mode_v: ImageAddressMode::Repeat,
                        anisotropy_clamp: 1, // 16,
                        ..ImageSamplerDescriptor::default()
                    });

                    // Set mipmap level
                    if use_mips {
                        texture.texture_descriptor.mip_level_count = mip_maps as u32 + 1;
                    }

                    texture
                });

                // Tex name, (task, Vec<(mat handle, tex type)>)
                (tex.get_name(), (task, Vec::new()))
            })
            .collect::<HashMap<_, _>>();

        let mut trans_map = HashMap::new(); // TODO: Clean this up
        let obj_keys = (0..milo.get_entries().len())
            .map(|_| state.get_next_obj_key())
            .collect::<Vec<_>>();

        for (i, obj) in milo.get_entries().iter().enumerate() {
            match obj {
                Object::BandPlacer(band_placer) => {
                    let mat = map_matrix(band_placer.get_local_xfm());

                    let placer_entity = commands
                        .spawn((
                            Name::new(band_placer.name.to_owned()),
                            Transform::from_matrix(mat),
                            Visibility::Inherited,
                            MiloObject {
                                id: obj_keys[i],
                                name: band_placer.name.to_owned(),
                                dir: obj_dir_name.to_owned(),
                            },
                            MiloBandPlacer
                        ))
                        .id();

                    if let Some(callback) = callback {
                        let mut entity_command = commands.entity(placer_entity);
                        callback(&mut entity_command);
                    }

                    commands
                        .entity(root_entity)
                        .add_child(placer_entity);

                    log::info!("Loaded band placer: {}", band_placer.get_name());
                },
                Object::Cam(cam) => {
                    let cam_entity = commands
                        .spawn((
                            Name::new(cam.name.to_owned()),
                            Camera3d::default(),
                            Camera {
                                is_active: false,
                                ..Default::default()
                            },
                            Projection::Perspective(
                                PerspectiveProjection {
                                    fov: cam.y_fov,
                                    aspect_ratio: 1.0,
                                    near: cam.near_plane,
                                    far: cam.far_plane
                                }
                            ),
                            Transform::from_matrix(
                                map_matrix(cam.get_local_xfm())
                            ), //.looking_at(Vec3::ZERO, Vec3::Z),
                            Visibility::Inherited,
                            MiloObject {
                                id: obj_keys[i],
                                name: cam.name.to_owned(),
                                dir: obj_dir_name.to_owned(),
                            },
                            MiloCam
                        ))
                        .id();

                    if let Some(callback) = callback {
                        let mut entity_command = commands.entity(cam_entity);
                        callback(&mut entity_command);
                    }

                    commands
                        .entity(root_entity)
                        .add_child(cam_entity);

                    log::info!("Loaded cam: {}", cam.get_name());
                },
                Object::CharClipSamples(ccs) => {
                    let mut anim_clip = AnimationClip::default();

                    let one_samples = match &ccs.one.samples {
                        EncodedSamples::Uncompressed(unc_samples) => unc_samples,
                        _ => &ccs.one.decode_samples(&sys_info),
                    };
                    let full_samples = match &ccs.full.samples {
                        EncodedSamples::Uncompressed(unc_samples) => unc_samples,
                        _ => &ccs.full.decode_samples(&sys_info),
                    };

                    let sample_count = ccs.one.get_sample_count().max(ccs.full.get_sample_count());

                    for sample in one_samples.into_iter().chain(full_samples) {
                        let bone_name = format!("{}.mesh", &sample.symbol);
                        let anim_target_id = AnimationTargetId::from_name(&bone_name.into());

                        if let Some((_, pos)) = sample.pos.as_ref() {
                            anim_clip
                                .add_curve_to_target(
                                    anim_target_id,
                                    AnimatableCurve::new(
                                        animated_field!(Transform::translation),
                                        AnimatableKeyframeCurve::new(pos
                                            .iter()
                                            .cycle()
                                            .take(sample_count)
                                            .enumerate()
                                            .map(|(i, p)| (i as f32, Vec3::new(p.x, p.y, p.z))))
                                        .expect("Failed to build translation curve")
                                    )
                                );
                        }

                        if let Some((_, quat)) = sample.quat.as_ref() {
                            anim_clip
                                .add_curve_to_target(
                                    anim_target_id,
                                    AnimatableCurve::new(
                                        animated_field!(Transform::rotation),
                                        AnimatableKeyframeCurve::new(quat
                                            .iter()
                                            .cycle()
                                            .take(sample_count)
                                            .enumerate()
                                            .map(|(i, q)| (i as f32, Quat::from_xyzw(q.x, q.y, q.z, q.w))))
                                        .expect("Failed to build rotation curve")
                                    )
                                );
                        }

                        // TODO: Combine rotx, roty, rotz samples
                        // Need to group one + full samples by target to accomplish
                        if let Some((_, rotz)) = sample.rotz.as_ref() {
                            anim_clip
                                .add_curve_to_target(
                                    anim_target_id,
                                    AnimatableCurve::new(
                                        animated_field!(Transform::rotation),
                                        AnimatableKeyframeCurve::new(rotz
                                            .iter()
                                            .cycle()
                                            .take(sample_count)
                                            .enumerate()
                                            .map(|(i, rz)| (i as f32, Quat::from_rotation_z(*rz * (std::f32::consts::PI / 180.0)))))
                                        .expect("Failed to build fragmented rotation curve (from x, y, z components")
                                    )
                                );
                        }

                        if let Some((_, scale)) = sample.scale.as_ref() {
                            anim_clip
                                .add_curve_to_target(
                                    anim_target_id,
                                    AnimatableCurve::new(
                                        animated_field!(Transform::scale),
                                        AnimatableKeyframeCurve::new(scale
                                            .iter()
                                            .cycle()
                                            .take(sample_count)
                                            .enumerate()
                                            .map(|(i, s)| (i as f32, Vec3::new(s.x, s.y, s.z))))
                                        .expect("Failed to build scale curve")
                                    )
                                );
                        }
                    }

                    //if !anim_clip.curves().is_empty() {}
                    let anim_entity = commands
                        .spawn((
                            Name::new(ccs.name.to_owned()),
                            MiloObject {
                                id: obj_keys[i],
                                name: ccs.name.to_owned(),
                                dir: obj_dir_name.to_owned(),
                            },
                            MiloCharClip(animations.add(anim_clip)),
                            ChildOf(root_entity)
                        ))
                        .id();

                    if let Some(callback) = callback {
                        let mut entity_command = commands.entity(anim_entity);
                        callback(&mut entity_command);
                    }

                    log::info!("Loaded char clip samples: {}", ccs.get_name());
                },
                Object::CharHair(ch) => {
                    continue;

                    for (si, strand) in ch.strands.iter().enumerate() {
                        // TODO: Don't forget about root field!
                        let root_bone = &strand.root;
                        let strand_mat = map_matrix3(&strand.base_mat);
                        let mut previous_origin = MILO_TO_BEVY_MATRIX * strand_mat.clone();

                        let strand_name = format!("{}/strand_{si}", ch.get_name());

                        let rad = 0.1;
                        let shift = 1.0;

                        let bone_entity = trans_map.get(root_bone).map(|b| *b).unwrap();

                        let strand_entity = commands
                            .spawn((
                                Name::new(strand_name),
                                //Transform::from_matrix(strand_mat),
                                Transform::IDENTITY,
                                Visibility::Inherited,
                                //CustomParent(root_bone.to_owned()),
                                CloneWorldTransform(bone_entity),
                                ParentOverride,
                                //ChildOf(root_entity),
                                MiloObject {
                                    id: obj_keys[i],
                                    name: ch.name.to_owned(),
                                    dir: obj_dir_name.to_owned(),
                                },
                                MiloCharHair,
                                RigidBody::KinematicPositionBased,
                                Collider::cuboid(rad, rad, rad)
                            ))
                            .id();

                        let mut point_parent_entity = strand_entity;

                        for (pi, point) in strand.points.iter().enumerate() {
                            let point_name = format!("{}/strand_{si}/point_{pi}", ch.get_name());
                            println!("ERRERJE EJRE {}", &point_name);

                            //let axis = Vec3::new(1.0, 1.0, 0.0);
                            let joint = SphericalJointBuilder::new()
                                .local_anchor1(Vec3::new(0.0, 0.0, 1.0))
                                .local_anchor2(Vec3::new(0.0, 0.0, -3.0));
                            let joint = RopeJointBuilder::new(point.length)
                                .local_anchor1(Vec3::splat(0.0))
                                .local_anchor2(Vec3::new(0.0, -point.length, 0.0));

                            let joint = SphericalJointBuilder::new()
                                .local_anchor1(Vec3::splat(0.0))
                                .local_anchor2(Vec3::new(0.0, -point.length, 0.0));

                            let point_entity = commands
                                .spawn((
                                    Name::new(point_name),
                                    Transform::from_matrix(previous_origin),
                                    //Transform::IDENTITY,
                                    //Transform::from_translation(Vec3::new(0.0, -point.length * 0.5, 0.0)),
                                    Visibility::Inherited,
                                    //ChildOf(point_parent_entity),
                                    ParentOverride,
                                    MiloObject {
                                        id: obj_keys[i],
                                        name: ch.name.to_owned(),
                                        dir: obj_dir_name.to_owned(),
                                    },
                                    MiloCharHair,
                                    RigidBody::Dynamic,
                                    //LockedAxes::TRANSLATION_LOCKED,
                                    //Collider::cuboid(rad, rad, rad),
                                    //Collider::ball(0.25),
                                    //Collider::cylinder(point.length * 0.5, 0.5),
                                    //ColliderMassProperties::Mass(20.),
                                    //AdditionalMassProperties::Mass(0.5),
                                    AdditionalMassProperties::MassProperties(MassProperties {
                                        mass: 1.0,
                                        local_center_of_mass: Vec3::new(0.0, -point.length * 0.5, 0.0),
                                        //local_center_of_mass: Vec3::new(0.0, -point.length, 0.0),
                                        principal_inertia: Vec3::splat(ch.inertia * 100.0),
                                        ..Default::default()
                                    }),
                                    Friction::coefficient(ch.friction),
                                    Restitution::coefficient(50.0),
                                    ImpulseJoint::new(point_parent_entity, joint),
                                    ColliderDisabled
                                ))
                                .with_child((
                                    Transform::from_translation(Vec3::new(0.0, -point.length * 0.5, 0.0)),
                                    Collider::cylinder(point.length * 0.4, 0.1),
                                    //ColliderMassProperties::Mass(20.),
                                ))
                                .id();

                            if let Some(callback) = callback {
                                let mut entity_command = commands.entity(point_entity);
                                callback(&mut entity_command);
                            }

                            if point_parent_entity != strand_entity {
                                commands
                                    .entity(point_parent_entity)
                                    .trigger(SetCloneWorldTransform(point_parent_entity, point.bone.to_owned()));
                            }

                            previous_origin *= Mat4::from_translation(Vec3::new(0., 0., point.length));
                            point_parent_entity = point_entity;
                        }

                        if let Some(callback) = callback {
                            let mut entity_command = commands.entity(strand_entity);
                            callback(&mut entity_command);
                        }
                    }

                    log::info!("Loaded char hair: {}", ch.get_name());
                },
                Object::Group(group) => {
                    let mat = map_matrix(group.get_local_xfm());

                    let group_entity = commands
                        .spawn((
                            Name::new(group.name.to_owned()),
                            Transform::from_matrix(mat),
                            OriginalTransform(Transform::from_matrix(mat)),
                            Visibility::Inherited,
                            MiloObject {
                                id: obj_keys[i],
                                name: group.name.to_owned(),
                                dir: obj_dir_name.to_owned(),
                            }
                        ))
                        .id();

                    if let Some(callback) = callback {
                        let mut entity_command = commands.entity(group_entity);
                        callback(&mut entity_command);
                    }

                    commands
                        .entity(root_entity)
                        .add_child(group_entity);

                    log::info!("Loaded group: {}", group.get_name());
                },
                Object::Mesh(mesh) => {
                    // Ignore meshes without geometry (used mostly in GH1)
                    if mesh.vertices.is_empty() || mesh.name.starts_with("shadow") {
                        continue;
                    }

                    let mat = map_matrix(mesh.get_local_xfm());

                    let mut bevy_mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList, bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD | bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD);

                    let vert_count = mesh.get_vertices().len();

                    let mut positions = vec![Default::default(); vert_count];
                    let mut normals = vec![Default::default(); vert_count];
                    let mut tangents = vec![Default::default(); vert_count];
                    let mut uvs = vec![Default::default(); vert_count];

                    for (i, vert) in mesh.get_vertices().iter().enumerate() {
                        positions[i] = [vert.pos.x, vert.pos.y, vert.pos.z];

                        // TODO: Figure out normals/tangents
                        //normals.push([vert.normals.x, vert.normals.y, vert.normals.z]);
                        normals[i] = [1.0, 1.0, 1.0];
                        tangents[i] = [0.0, 0.0, 0.0, 1.0];

                        uvs[i] = [vert.uv.u, vert.uv.v];
                    }

                    let indices = bevy::render::mesh::Indices::U16(
                        mesh.faces.iter().flat_map(|f| *f).collect()
                    );

                    bevy_mesh.insert_indices(indices);
                    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
                    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

                    let milo_mat = milo_materials.get(&mesh.mat);

                    let bevy_mat = match milo_mat {
                        Some(mat) => StandardMaterial {
                            //alpha_mode: AlphaMode::Blend,
                            alpha_mode: match (mat.blend, mat.z_mode) {
                                (Blend::kBlendSrcAlpha, ZMode::kZModeDisable) => AlphaMode::Blend,
                                _ => AlphaMode::Opaque
                            },
                            base_color: Color::srgba(
                                mat.color.r,
                                mat.color.g,
                                mat.color.b,
                                mat.alpha,
                            ),
                            double_sided: true,
                            unlit: true,
                            base_color_texture: None,
                            normal_map_texture: None,
                            emissive_texture: None,
                            //roughness: 0.8, // TODO: Bevy 0.6 migration
                            /*base_color_texture: get_texture(&mut loader, &mat.diffuse_tex, system_info)
                                .and_then(map_texture)
                                .and_then(|t| Some(bevy_textures.add(t))),
                            normal_map: get_texture(&mut loader, &mat.norm_detail_map, system_info)
                                .and_then(map_texture)
                                .and_then(|t| Some(bevy_textures.add(t))),
                            emissive_texture: get_texture(&mut loader, &mat.emissive_map, system_info)
                                .and_then(map_texture)
                                .and_then(|t| Some(bevy_textures.add(t))),*/
                            ..Default::default()
                        },
                        None => StandardMaterial {
                            base_color: Color::srgb(0.3, 0.5, 0.3),
                            double_sided: true,
                            unlit: false,
                            ..Default::default()
                        },
                    };

                    let mat_handle = materials.add(bevy_mat);

                    let textures = [
                        (milo_mat.map(|mat| &mat.diffuse_tex), TextureType::Diffuse),
                        (milo_mat.map(|mat| &mat.normal_map), TextureType::Normal),
                        (milo_mat.map(|mat| &mat.emissive_map), TextureType::Emissive),
                    ];

                    for (tex, typ) in textures.into_iter() {
                        let Some((_, mats)) = tex.and_then(|t| textures_to_load.get_mut(t)) else {
                            continue
                        };

                        mats.push((mat_handle.clone(), typ));
                    }

                    // Add mesh
                    let mesh_entity = commands
                        .spawn((
                            Name::new(mesh.name.to_owned()),
                            Mesh3d(meshes.add(bevy_mesh)),
                            MeshMaterial3d(mat_handle),
                            Transform::from_matrix(mat),
                            OriginalTransform(Transform::from_matrix(mat)),
                            Visibility::Inherited,
                            MiloObject {
                                id: obj_keys[i],
                                name: mesh.name.to_owned(),
                                dir: obj_dir_name.to_owned(),
                            },
                            MiloMesh {
                                verts: mesh.vertices.len(),
                                faces: mesh.faces.len()
                            },
                            /*AnimationTarget {
                                id:  AnimationTargetId::from_name(&mesh.name.to_owned().into()),
                                player: root_entity,
                            }*/
                        ))
                        .id();

                    if mesh.sphere.r > 0.0 && false {
                        // Create sphere (TODO: Spawn from shared mesh)
                        let MiloSphere { x, y, z, r } = &mesh.sphere;

                        log::debug!("Adding sphere to {} with radius {}", &mesh.name, *r);

                        let sphere_entity = commands
                            .spawn((
                                Mesh3d(meshes.add(Sphere::new(*r).mesh())),
                                MeshMaterial3d(materials.add({
                                    let mut mat: StandardMaterial = Color::srgb(0.9, 0.9, 0.9).into();

                                    mat.unlit = true;

                                    mat
                                })),
                                Transform::from_xyz(*x, *y, *z),
                                Visibility::Inherited
                            ))
                            .id();

                        // Add sphere
                        commands
                            .entity(mesh_entity)
                            .add_child({
                                sphere_entity

                                /*commands
                                    .spawn(Sphere {
                                        center: Vec3A::new(*x, *y, *z),
                                        radius: *r,
                                    })
                                    .id()*/
                            });
                    }

                    if let Some(callback) = callback {
                        let mut entity_command = commands.entity(mesh_entity);
                        callback(&mut entity_command);
                    }

                    commands
                        .entity(root_entity)
                        .add_child(mesh_entity);

                    log::info!("Loaded mesh: {}", mesh.get_name());
                },
                Object::Trans(trans) => {
                    let mat = map_matrix(trans.get_local_xfm());

                    let trans_entity = commands
                        .spawn((
                            Name::new(trans.name.to_owned()),
                            Transform::from_matrix(mat),
                            OriginalTransform(Transform::from_matrix(mat)),
                            Visibility::Inherited,
                            MiloObject {
                                id: obj_keys[i],
                                name: trans.name.to_owned(),
                                dir: obj_dir_name.to_owned(),
                            },
                            //RigidBody::Dynamic,
                            //LockedAxes::TRANSLATION_LOCKED,
                            MiloBone,
                            /*AnimationTarget {
                                id:  AnimationTargetId::from_name(&trans.name.to_owned().into()),
                                player: root_entity,
                            }*/
                        ))
                        .id();

                    if let Some(callback) = callback {
                        let mut entity_command = commands.entity(trans_entity);
                        callback(&mut entity_command);
                    }

                    commands
                        .entity(root_entity)
                        .add_child(trans_entity);

                    log::info!("Loaded trans: {}", trans.get_name());
                    trans_map.insert(trans.get_name(), trans_entity);
                },
                _ => {}
            }
        }

        // Add image tasks to components
        for (name, (task, mats)) in textures_to_load.into_iter() {
            commands
                .spawn(MiloAsyncTexture {
                    tex_name: name.to_owned(),
                    image_task: task,
                    mat_handles: mats
                });
        }

        let is_character = obj_dir_type.eq("Character");
        if is_character {
            update_skinned_meshes_events_writer.write(UpdateSkinnedMeshes(obj_dir_name.to_owned()));
        }

        milos_updated = true;

        for (milo_entry, key) in milo.get_entries_mut().drain(..).zip(obj_keys) {
            state.objects.insert(key, milo_entry);
        }

        // TODO: Remove useless event
        //scene_events_writer.write(LoadMiloSceneComplete(milo_path.to_owned()));
    }

    milos_updated
}

fn update_milo_object_parents(
    milos_updated: In<bool>,
    mut commands: Commands,
    state: Res<MiloState>,
    root_query: Query<Entity, With<MiloRoot>>,
    milo_objects_query: Query<(Entity, &MiloObject, Has<ParentOverride>, Option<&CustomParent>)>,
) {
    if !*milos_updated {
        return;
    }

    log::debug!("Updating parents!");

    let root_entity = root_query.single().unwrap(); // TODO: Handle better

    let obj_entities = milo_objects_query
        .iter()
        .map(|(en, mo, hpo, cp)| (en, state.objects.get(&mo.id).unwrap(), hpo, cp))
        .collect::<Vec<_>>();

    /*let (entity_map, children_map) = obj_entities
        .iter()
        .map(|(en, mo)| (en, &state.objects[mo.id as usize]))
        .fold((HashMap::new(), HashMap::new()), |(mut entity_acc, mut children_acc), (en, obj)| {
            entity_acc.insert(obj.get_name(), en.clone());

            let trans_parent = match obj {
                Object::BandPlacer(obj) => &obj.parent,
                Object::Cam(obj) => &obj.parent,
                Object::Mesh(obj) => &obj.parent,
                Object::Group(obj) => &obj.parent,
                Object::Trans(obj) => &obj.parent,
                _ => {
                    return (entity_acc, children_acc);
                }
            };

            if trans_parent.is_empty() || trans_parent.eq(obj.get_name()) {
                return (entity_acc, children_acc);
            }

            children_acc
                .entry(trans_parent.as_str())
                .and_modify(|ch: &mut Vec<_>| ch.push(obj.get_name()))
                .or_insert_with(|| vec![obj.get_name()]);

            (entity_acc, children_acc)
        });*/

    log::debug!("Found {}/{} objects!", obj_entities.len(), state.objects.len());

    let (entity_map, parent_map) = obj_entities
        .iter()
        .fold((HashMap::new(), HashMap::new()), |(mut entity_acc, mut parent_acc), (en, obj, _, cp)| {
            entity_acc.insert(obj.get_name(), en.clone());

            // TODO: Clean this up
            if let Some(&CustomParent(parent)) = cp.as_ref() {
                if !parent.is_empty() && !parent.eq(obj.get_name()) {
                    parent_acc.insert(obj.get_name(), parent.as_str());
                }
            }

            let trans_parent = match obj {
                Object::BandPlacer(obj) => &obj.parent,
                Object::Cam(obj) => &obj.parent,
                Object::Mesh(obj) => &obj.parent,
                Object::Group(obj) => &obj.parent,
                Object::Trans(obj) => &obj.parent,
                _ => {
                    return (entity_acc, parent_acc);
                }
            };

            if !trans_parent.is_empty() && !trans_parent.eq(obj.get_name()) {
                parent_acc.insert(obj.get_name(), trans_parent.as_str());
            }

            (entity_acc, parent_acc)
        });

    for (entity, obj, parent_override, _) in obj_entities {
        if parent_override {
            continue;
        }

        // Clear parent
        /*commands
            .entity(entity)
            .remove_parent();*/

        // Set new parent
        // Get trans parent or use root
        let new_parent_entity = parent_map
            .get(obj.get_name())
            .and_then(|o| {
                let parent_entity = entity_map.get(o);
                let entity = parent_entity.map(|e| e.clone());

                if entity.is_none() {
                    log::warn!("Can't find trans for \"{}\"", o);
                }

                entity
            })
            .unwrap_or(root_entity);

        commands
            .entity(entity)
            .insert(ChildOf(new_parent_entity));
    }
}

fn update_skinned_meshes(
    mut update_skinned_meshes_events_reader: EventReader<UpdateSkinnedMeshes>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut skinned_mesh_inverse_bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    state: Res<MiloState>,
    bone_query: Query<(Entity, &MiloObject), With<MiloBone>>,
    mesh_query: Query<(Entity, &Mesh3d, &MiloObject), (With<MiloMesh>, Without<SkinnedMesh>)>,
) {
    for UpdateSkinnedMeshes(obj_dir_name) in update_skinned_meshes_events_reader.read() {
        let transforms_map = bone_query
            .iter()
            .filter(|(_, mo)| mo.dir.eq(obj_dir_name))
            .map(|(_, mo)| {
                // TODO: Support GH1-style bones
                let trans = match state.objects.get(&mo.id) {
                    Some(Object::Trans(trans)) => trans as &dyn Trans,
                    _ => unreachable!("Bone object not found"),
                };

                (&mo.name, trans)
            })
            .collect::<HashMap<_, _>>();

        let (poses, bone_idx_map) = bone_query
            .iter()
            .filter(|(_, mo)| mo.dir.eq(obj_dir_name))
            .map(|(e, mo)| {
                let global_mat = compute_global_mat(&mo.name, &transforms_map);
                (&mo.name, e, global_mat.inverse())
            })
            .fold((Vec::new(), HashMap::new()), |(mut poses, mut bone_map), (name, e, mat)| {
                bone_map.insert(name, (e, poses.len()));
                poses.push(mat);

                (poses, bone_map)
            });

        //let inverse_bindposes = skinned_mesh_inverse_bindposes.add(poses);

        for (en, Mesh3d(mesh_handle), mo) in mesh_query.iter() {
            let mesh = meshes.get_mut(mesh_handle).unwrap();
            let milo_mesh = match state.objects.get(&mo.id) {
                Some(Object::Mesh(mesh)) => mesh,
                _ => unreachable!("Mesh object not found"),
            };

            let is_skinned = milo_mesh
                .bones
                .iter()
                .any(|b| !b.name.is_empty());

            if !is_skinned {
                log::error!("Not skinned: {}", &mo.name);
                continue;
            }

            let (local_poses, local_bone_entities) = milo_mesh
                .bones
                .iter()
                .map(|b| bone_idx_map
                    .get(&b.name)
                    .map(|(e, global_i)| (*e, /*poses[*global_i]*/ map_matrix(&b.trans)))
                    .unwrap_or((Entity::PLACEHOLDER, Mat4::IDENTITY))
                )
                .fold((Vec::new(), Vec::new()), |(mut poses, mut entities), (en, mat)| {
                    poses.push(mat);
                    entities.push(en);

                    (poses, entities)
                });

            mesh.insert_attribute(
                Mesh::ATTRIBUTE_JOINT_INDEX,
                VertexAttributeValues::Uint16x4(
                    milo_mesh.vertices.iter().map(|v| v.bones.clone()).collect::<Vec<_>>()
                )
            );

            mesh.insert_attribute(
                Mesh::ATTRIBUTE_JOINT_WEIGHT,
                milo_mesh.vertices.iter().map(|v| v.weights.clone()).collect::<Vec<_>>()
            );

            let local_inverse_bindposes = skinned_mesh_inverse_bindposes.add(local_poses);

            commands
                .entity(en)
                .insert(SkinnedMesh {
                    inverse_bindposes: local_inverse_bindposes,
                    joints: local_bone_entities
                });
        }

        log::debug!("Loaded skin for {}", obj_dir_name);
    }
}

fn update_skinned_meshes_ik(
    mut update_skinned_meshes_events_reader: EventReader<UpdateSkinnedMeshes>,
    mut commands: Commands,
    bone_query: Query<(Entity, &MiloObject), With<MiloBone>>,
) {
    for UpdateSkinnedMeshes(obj_dir_name) in update_skinned_meshes_events_reader.read() {
        let bone_map = bone_query
            .iter()
            .filter(|(_, mo)| mo.dir.eq(obj_dir_name))
            .map(|(en, mo)| (mo.name.as_str(), en))
            .collect::<HashMap<_, _>>();

        //let hand_l = bone_map.get("bone_L-hand.mesh").unwrap();
        //let foretwist2_l = bone_map.get("bone_L-foreTwist2.mesh").unwrap();

        // TODO: Clean all this up into something nicer
        let Some(upperarm_l) = bone_map.get("bone_L-upperArm.mesh") else {
            continue;
        };
        let Some(uppertwist_l) = bone_map.get("bone_L-upperTwist1.mesh") else {
            continue;
        };

        let Some(forearm_l) = bone_map.get("bone_L-foreArm.mesh") else {
            continue;
        };
        let Some(foretwist1_l) = bone_map.get("bone_L-foreTwist1.mesh") else {
            continue;
        };

        commands
            .entity(*uppertwist_l)
            .insert(CloneTransform(*upperarm_l));

            commands
            .entity(*foretwist1_l)
            .insert(CloneTransform(*forearm_l));

        let Some(upperarm_r) = bone_map.get("bone_R-upperArm.mesh") else {
            continue;
        };
        let Some(uppertwist_r) = bone_map.get("bone_R-upperTwist1.mesh") else {
            continue;
        };

        let Some(forearm_r) = bone_map.get("bone_R-foreArm.mesh") else {
            continue;
        };
        let Some(foretwist1_r) = bone_map.get("bone_R-foreTwist1.mesh") else {
            continue;
        };

        commands
            .entity(*uppertwist_r)
            .insert(CloneTransform(*upperarm_r));

        commands
            .entity(*foretwist1_r)
            .insert(CloneTransform(*forearm_r));

        /*commands
            .entity(*foretwist2_l)
            .insert(IkConstraint {
                chain_length: 3,
                iterations: 20,
                target: *hand_l,
                pole_target: None,
                pole_angle: std::f32::consts::FRAC_PI_2,
                enabled: true,
            });*/
    }
}

fn clone_transforms(
    transforms_query: Query<&Transform, (Without<CloneTransform>, Without<CloneWorldTransform>)>,
    mut entity_query: Query<(&mut Transform, &CloneTransform)>,
) {
    for (mut transform, &CloneTransform(clone_en)) in entity_query.iter_mut() {
        let clone_transform = transforms_query.get(clone_en).unwrap();
        *transform = *clone_transform;
    }
}

fn clone_world_transforms(
    transforms_query: Query<&GlobalTransform, (Without<CloneTransform>, Without<CloneWorldTransform>)>,
    mut entity_query: Query<(&mut Transform, &CloneWorldTransform), Without<PhysicsControlledBone>>,
) {
    for (mut transform, &CloneWorldTransform(clone_en)) in entity_query.iter_mut() {
        let clone_transform = transforms_query.get(clone_en).unwrap();
        *transform = clone_transform.compute_transform();
    }
}

fn clone_world_transforms_with_physics(
    transforms_query: Query<&GlobalTransform, (Without<CloneTransform>, Without<CloneWorldTransform>)>,
    mut entity_query: Query<(&mut Transform, &CloneWorldTransform), With<PhysicsControlledBone>>,
) {
    for (mut transform, &CloneWorldTransform(clone_en)) in entity_query.iter_mut() {
        let clone_transform = transforms_query.get(clone_en).unwrap();
        *transform = clone_transform.compute_transform();
    }
}

fn compute_global_mat(
    bone_name: &String,
    transforms: &HashMap<&String, &dyn Trans>
) -> Mat4 {
    match transforms.get(bone_name) {
        Some(trans) if !trans.get_parent().is_empty()
            => compute_global_mat(trans.get_parent(), transforms) * map_matrix(trans.get_local_xfm()),
        Some(trans) => map_matrix(trans.get_local_xfm()),
        _ => Mat4::IDENTITY,
    }
}

fn process_milo_async_textures(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut task_query: Query<(Entity, &mut MiloAsyncTexture)>,
) {
    for (entity, mut async_tex) in &mut task_query {
        if let Some(img) = future::block_on(future::poll_once(&mut async_tex.image_task)) {
            // Add texture
            let img_handle = images.add(img);

            // Update material
            for (mat_handle, tex_type) in async_tex.mat_handles.iter() {
                let mat = materials.get_mut(mat_handle).unwrap(); // Shouldn't fail

                let tex_field = match tex_type {
                    TextureType::Diffuse => &mut mat.base_color_texture,
                    TextureType::Normal => &mut mat.normal_map_texture,
                    TextureType::Emissive => &mut mat.emissive_texture
                };

                *tex_field = Some(img_handle.clone());
            }

            // Remove task entity
            commands
                .entity(entity)
                .despawn();

            log::info!("Loaded texture: {}", &async_tex.tex_name);
        }
    }
}

pub fn map_matrix(m: &Matrix) -> Mat4 {
    Mat4::from_cols_array(&[
        m.m11,
        m.m12,
        m.m13,
        m.m14,
        m.m21,
        m.m22,
        m.m23,
        m.m24,
        m.m31,
        m.m32,
        m.m33,
        m.m34,
        m.m41,
        m.m42,
        m.m43,
        m.m44,
    ])
}

pub fn map_matrix3(m: &Matrix3) -> Mat4 {
    Mat4::from_cols_array(&[
        m.m11,
        m.m12,
        m.m13,
        0.0,
        m.m21,
        m.m22,
        m.m23,
        0.0,
        m.m31,
        m.m32,
        m.m33,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0
    ])
}

fn image_new(
    size: Extent3d,
    dimension: TextureDimension,
    pixel: &[u8],
    format: TextureFormat,
) -> Image {
    // Problematic!!!
    /*debug_assert_eq!(
        size.volume() * format.pixel_size(),
        data.len(),
        "Pixel data, size and format have to match",
    );*/
    let mut image = Image {
        data: Some(pixel.to_owned()),
        ..Default::default()
    };
    image.texture_descriptor.dimension = dimension;
    image.texture_descriptor.size = size;
    image.texture_descriptor.format = format;
    image
}

fn image_new_fill(
    size: Extent3d,
    dimension: TextureDimension,
    pixel: &[u8],
    format: TextureFormat,
) -> Image {
    let mut value = Image::default();
    value.texture_descriptor.format = format;
    value.texture_descriptor.dimension = dimension;
    value.resize(size);

    // Problematic!!!
    /*debug_assert_eq!(
        pixel.len() % format.pixel_size(),
        0,
        "Must not have incomplete pixel data."
    );
    debug_assert!(
        pixel.len() <= value.data.len(),
        "Fill data must fit within pixel buffer."
    );*/

    // TODO: Refactor w/o unwrap
    for current_pixel in value.data.as_mut().unwrap().chunks_exact_mut(pixel.len()) {
        current_pixel.copy_from_slice(pixel);
    }
    value
}

// TODO: Load textures async