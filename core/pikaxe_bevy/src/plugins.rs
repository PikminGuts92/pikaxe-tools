use crate::prelude::*;
use bevy::animation::{animated_field, AnimationTargetId};
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::AsyncComputeTaskPool;
use std::collections::{HashMap, HashSet};
use futures_lite::future;
use pikaxe::ark::Ark;
use pikaxe::scene::{Blend, Matrix, MiloObject as MObject, Object, ObjectDir, Sphere as MiloSphere, Trans, RndMesh, ZMode};
use pikaxe::texture::Bitmap;
use pikaxe::Platform;
use std::path::PathBuf;

#[derive(Default)]
pub struct MiloPlugin {
    pub ark_path: Option<PathBuf>,
    pub default_outfit: Option<String>,
}

impl Plugin for MiloPlugin {
    fn build(&self, app: &mut App) {
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
        app.add_event::<LoadMiloSceneComplete>();
        app.add_event::<UpdateMiloObjectParents>();
        app.add_event::<UpdateSkinnedMeshes>();

        app.insert_resource(state);

        app.add_systems(Startup, init_world);

        app.add_systems(Update, (
            process_milo_scene_events,
            update_milo_object_parents,
            update_skinned_meshes,
        ).chain());

        app.add_systems(Update, process_milo_async_textures);
    }
}

fn init_world(
    mut commands: Commands,
) {
    // Translate to bevy coordinate system
    let trans_mat = Mat4::from_cols_array(&[
        -1.0,  0.0,  0.0, 0.0,
        0.0,  0.0,  1.0, 0.0,
        0.0,  1.0,  0.0, 0.0,
        0.0,  0.0,  0.0, 1.0,
    ]);

    //let scale_mat = Mat4::from_scale(Vec3::new(0.1, 0.1, 0.1));
    let trans = Transform::from_matrix(trans_mat);

    commands
        .spawn(Name::new("Root"))
        .insert((
            trans,
            GlobalTransform::from(trans), // TODO: Possibly remove
            Visibility::Inherited
        ))
        .insert(MiloRoot);
}

// TODO: Move to separate file?
fn process_milo_scene_events(
    mut commands: Commands,
    mut scene_events_reader: EventReader<LoadMiloScene>,
    mut scene_events_reader_commands: EventReader<LoadMiloSceneWithCommands>,
    mut state: ResMut<MiloState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut scene_events_writer: EventWriter<LoadMiloSceneComplete>,
    mut update_parents_events_writer: EventWriter<UpdateMiloObjectParents>,
    mut update_skinned_meshes_events_writer: EventWriter<UpdateSkinnedMeshes>,
    root_query: Query<Entity, With<MiloRoot>>,
) {
    /*for e in state.ark.as_ref().unwrap().entries.iter() {
        log::debug!("{}", &e.path);
    }*/

    let thread_pool = AsyncComputeTaskPool::get();
    let root_entity = root_query.single().unwrap(); // TODO: Handle safely

    let mut milos_updated = false;

    let scene_events = scene_events_reader
        .read()
        .map(|LoadMiloScene(p)| (p, None))
        .chain(scene_events_reader_commands
            .read()
            .map(|LoadMiloSceneWithCommands(p, c)| (p, Some(c)))
        );

    // TODO: Check if path ends in .milo
    for (milo_path, callback) in scene_events {
        let start_idx = state.objects.len();
        log::debug!("Loading Scene: \"{}\"", milo_path);

        let (sys_info, mut milo) = state.open_milo(&milo_path).unwrap();

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
                                id: (start_idx + i) as u32,
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
                                id: (start_idx + i) as u32,
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

                    let one_samples = ccs.one.decode_samples(&sys_info);
                    let full_samples = ccs.full.decode_samples(&sys_info);

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
                                id: (start_idx + i) as u32,
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
                },
                Object::Group(group) => {
                    let mat = map_matrix(group.get_local_xfm());

                    let group_entity = commands
                        .spawn((
                            Name::new(group.name.to_owned()),
                            Transform::from_matrix(mat),
                            Visibility::Inherited,
                            MiloObject {
                                id: (start_idx + i) as u32,
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
                            Visibility::Inherited,
                            MiloObject {
                                id: (start_idx + i) as u32,
                                name: mesh.name.to_owned(),
                                dir: obj_dir_name.to_owned(),
                            },
                            MiloMesh {
                                verts: mesh.vertices.len(),
                                faces: mesh.faces.len()
                            }
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
                            Visibility::Inherited,
                            MiloObject {
                                id: (start_idx + i) as u32,
                                name: trans.name.to_owned(),
                                dir: obj_dir_name.to_owned(),
                            },
                            MiloBone
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
        state.objects.append(milo.get_entries_mut());
        scene_events_writer.write(LoadMiloSceneComplete(milo_path.to_owned()));
    }

    if milos_updated {
        update_parents_events_writer.write(UpdateMiloObjectParents);
    }
}

fn update_milo_object_parents(
    mut commands: Commands,
    state: Res<MiloState>,
    root_query: Query<Entity, With<MiloRoot>>,
    milo_objects_query: Query<(Entity, &MiloObject, Has<ParentOverride>), With<Transform>>,
    update_parents_events_reader: EventReader<UpdateMiloObjectParents>,
) {
    if update_parents_events_reader.is_empty() {
        return;
    }

    log::debug!("Updating parents!");

    let root_entity = root_query.single().unwrap(); // TODO: Handle better

    let obj_entities = milo_objects_query
        .iter()
        .map(|(en, mo, hpo)| (en, &state.objects[mo.id as usize], hpo))
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
        .fold((HashMap::new(), HashMap::new()), |(mut entity_acc, mut parent_acc), (en, obj, _)| {
            entity_acc.insert(obj.get_name(), en.clone());

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

    for (entity, obj, parent_override) in obj_entities {
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
                let trans = match state.objects.get(mo.id as usize) {
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

        let inverse_bindposes = skinned_mesh_inverse_bindposes.add(poses);

        for (en, Mesh3d(mesh_handle), mo) in mesh_query.iter() {
            let mesh = meshes.get_mut(mesh_handle).unwrap();
            let milo_mesh = match state.objects.get(mo.id as usize) {
                Some(Object::Mesh(mesh)) => mesh,
                _ => unreachable!("Mesh object not found"),
            };

            let is_skinned = milo_mesh
                .bones
                .iter()
                .any(|b| !b.name.is_empty());

            if !is_skinned {
                continue;
            }

            
        }

        log::debug!("Loaded skin for {}", obj_dir_name);
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