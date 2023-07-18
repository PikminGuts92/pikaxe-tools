use crate::prelude::*;
use bevy::prelude::*;
use bevy::render::render_resource::{AddressMode, Extent3d, SamplerDescriptor, TextureDimension, TextureFormat};
use bevy::render::texture::ImageSampler;
use bevy::tasks::AsyncComputeTaskPool;
use std::collections::{HashMap, HashSet};
use futures_lite::future;
use pikaxe::ark::Ark;
use pikaxe::scene::{Blend, Matrix, MiloObject as MObject, Object, ObjectDir, Trans, RndMesh, ZMode};
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

        app.insert_resource(state);

        app.add_systems(Startup, init_world);

        app.add_systems(Update, (
            process_milo_scene_events,
            apply_deferred,
            update_milo_object_parents
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
        .insert(SpatialBundle {
            transform: trans,
            global_transform: GlobalTransform::from(trans),
            ..Default::default()
        })
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
    mut scene_events_writer: EventWriter<LoadMiloSceneComplete>,
    mut update_parents_events_writer: EventWriter<UpdateMiloObjectParents>,
    root_query: Query<Entity, With<MiloRoot>>,
) {
    /*for e in state.ark.as_ref().unwrap().entries.iter() {
        log::debug!("{}", &e.path);
    }*/

    let thread_pool = AsyncComputeTaskPool::get();
    let root_entity = root_query.single();

    let mut milos_updated = false;

    let scene_events = scene_events_reader
        .iter()
        .map(|LoadMiloScene(p)| (p, None))
        .chain(scene_events_reader_commands
            .iter()
            .map(|LoadMiloSceneWithCommands(p, c)| (p, Some(c)))
        );

    // TODO: Check if path ends in .milo
    for (milo_path, callback) in scene_events {
        let start_idx = state.objects.len();
        log::debug!("Loading Scene: \"{}\"", milo_path);

        let (sys_info, mut milo) = state.open_milo(&milo_path).unwrap();

        let obj_dir_name = match &milo {
            ObjectDir::ObjectDir(dir) => &dir.name
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
                    texture.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
                        address_mode_u: AddressMode::Repeat,
                        address_mode_v: AddressMode::Repeat,
                        anisotropy_clamp: 1, // 16,
                        ..SamplerDescriptor::default()
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
                        .spawn(Name::new(band_placer.name.to_owned()))
                        .insert(SpatialBundle {
                            transform: Transform::from_matrix(mat),
                            ..Default::default()
                        })
                        .insert(MiloObject {
                            id: (start_idx + i) as u32,
                            name: band_placer.name.to_owned(),
                            dir: obj_dir_name.to_owned(),
                        })
                        .insert(MiloBandPlacer)
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
                        .spawn(Name::new(cam.name.to_owned()))
                        .insert(Camera3dBundle {
                            camera: Camera {
                                is_active: false,
                                ..Default::default()
                            },
                            transform: Transform::from_matrix(
                                map_matrix(cam.get_local_xfm())
                            ), //.looking_at(Vec3::ZERO, Vec3::Z),
                            projection: Projection::Perspective(
                                PerspectiveProjection {
                                    fov: cam.y_fov,
                                    aspect_ratio: 1.0,
                                    near: cam.near_plane,
                                    far: cam.far_plane
                                }
                            ),
                            ..Default::default()
                        })
                        .insert(MiloObject {
                            id: (start_idx + i) as u32,
                            name: cam.name.to_owned(),
                            dir: obj_dir_name.to_owned(),
                        })
                        .insert(MiloCam)
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
                Object::Group(group) => {
                    let mat = map_matrix(group.get_local_xfm());

                    let group_entity = commands
                        .spawn(Name::new(group.name.to_owned()))
                        .insert(SpatialBundle {
                            transform: Transform::from_matrix(mat),
                            ..Default::default()
                        })
                        .insert(MiloObject {
                            id: (start_idx + i) as u32,
                            name: group.name.to_owned(),
                            dir: obj_dir_name.to_owned(),
                        })
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

                    let mut bevy_mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);

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

                    bevy_mesh.set_indices(Some(indices));
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
                            base_color: Color::rgba(
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
                            base_color: Color::rgb(0.3, 0.5, 0.3),
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
                        .spawn(Name::new(mesh.name.to_owned()))
                        .insert(PbrBundle {
                            mesh: meshes.add(bevy_mesh),
                            material: mat_handle,
                            transform: Transform::from_matrix(mat),
                            ..Default::default()
                        })
                        .insert(MiloObject {
                            id: (start_idx + i) as u32,
                            name: mesh.name.to_owned(),
                            dir: obj_dir_name.to_owned(),
                        })
                        .insert(MiloMesh {
                            verts: mesh.vertices.len(),
                            faces: mesh.faces.len()
                        })
                        .id();

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
                        .spawn(Name::new(trans.name.to_owned()))
                        .insert(SpatialBundle {
                            transform: Transform::from_matrix(mat),
                            ..Default::default()
                        })
                        .insert(MiloObject {
                            id: (start_idx + i) as u32,
                            name: trans.name.to_owned(),
                            dir: obj_dir_name.to_owned(),
                        })
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

        milos_updated = true;
        state.objects.append(milo.get_entries_mut());
        scene_events_writer.send(LoadMiloSceneComplete(milo_path.to_owned()));
    }

    if milos_updated {
        update_parents_events_writer.send(UpdateMiloObjectParents);
    }
}

fn update_milo_object_parents(
    mut commands: Commands,
    state: Res<MiloState>,
    root_query: Query<Entity, With<MiloRoot>>,
    milo_objects_query: Query<(Entity, &MiloObject), With<Transform>>,
    mut update_parents_events_reader: EventReader<UpdateMiloObjectParents>,
) {
    if !update_parents_events_reader.iter().any(|_| true) {
        return;
    }

    log::debug!("Updating parents!");

    let root_entity = root_query.single();

    let obj_entities = milo_objects_query
        .iter()
        .map(|(en, mo)| (en, &state.objects[mo.id as usize]))
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
        .fold((HashMap::new(), HashMap::new()), |(mut entity_acc, mut parent_acc), (en, obj)| {
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

    for (entity, obj) in obj_entities {
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
            .set_parent(new_parent_entity);
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
        data: pixel.to_owned(),
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

    for current_pixel in value.data.chunks_exact_mut(pixel.len()) {
        current_pixel.copy_from_slice(pixel);
    }
    value
}

// TODO: Load textures async