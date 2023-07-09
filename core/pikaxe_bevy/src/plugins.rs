use crate::prelude::*;
use bevy::prelude::*;
use pikaxe::ark::{Ark, ArkOffsetEntry};
use pikaxe::io::*;
use pikaxe::scene::{CamObject, Matrix, Object, ObjectDir, Trans, RndMesh};
use pikaxe::SystemInfo;
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

        app.insert_resource(state);

        app.add_startup_system(init_world);
        app.add_system(process_milo_scene_events);
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
        .spawn_empty()
        .insert(TransformBundle {
            local: trans,
            global: GlobalTransform::from(trans)
        })
        .insert(VisibilityBundle::default())
        .insert(MiloRoot);
}

// TODO: Move to separate file?
fn process_milo_scene_events(
    mut commands: Commands,
    mut scene_events_reader: EventReader<LoadMiloScene>,
    mut state: ResMut<MiloState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    root_query: Query<Entity, With<MiloRoot>>,
) {
    /*for e in state.ark.as_ref().unwrap().entries.iter() {
        log::debug!("{}", &e.path);
    }*/

    let root_entity = root_query.single();

    // TODO: Check if path ends in .milo
    for LoadMiloScene(milo_path) in scene_events_reader.iter() {
        log::debug!("Loading Scene: \"{}\"", milo_path);

        let ark = state.ark.as_ref().unwrap();
        let mut milo = open_milo(ark, &milo_path).unwrap();

        for obj in milo.get_entries() {
            match obj {
                Object::Cam(cam) => {
                    let cam_entity = commands
                        .spawn_empty()
                        .insert(Camera3dBundle {
                            camera: Camera {
                                is_active: false,
                                ..Default::default()
                            },
                            transform: Transform::from_matrix(
                                map_matrix(cam.get_world_xfm()) // TODO: Use local instead
                            ).looking_at(Vec3::ZERO, Vec3::Z),
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
                            id: 0, // TODO: Assign unique id
                            name: cam.name.to_owned(),
                        })
                        .id();

                    commands
                        .entity(root_entity)
                        .add_child(cam_entity);
                },
                Object::Mesh(mesh) => {
                    // Ignore meshes without geometry (used mostly in GH1)
                    if mesh.vertices.is_empty() || mesh.name.starts_with("shadow") {
                        continue;
                    }

                    // Get transform (TODO: Get full computed?)
                    let mat = map_matrix(mesh.get_world_xfm());

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

                    // TODO: Map texture

                    let bevy_mat = StandardMaterial {
                        base_color: Color::rgb(0.3, 0.5, 0.3),
                        double_sided: true,
                        unlit: false,
                        ..Default::default()
                    };

                    // Add mesh
                    let mesh_entity = commands
                        .spawn(PbrBundle {
                            mesh: meshes.add(bevy_mesh),
                            material: materials.add(bevy_mat),
                            transform: Transform::from_matrix(mat),
                            ..Default::default()
                        })
                        .insert(MiloObject {
                            id: 0, // TODO: Assign unique id
                            name: mesh.name.to_owned(),
                        })
                        .insert(MiloMesh {
                            verts: mesh.vertices.len(),
                            faces: mesh.faces.len()
                        })
                        .id();

                    commands
                        .entity(root_entity)
                        .add_child(mesh_entity);
                },
                _ => {}
            }
        }

        state.objects.append(milo.get_entries_mut());
    }
}

fn open_milo(ark: &Ark, milo_path: &str) -> Option<ObjectDir> {
    let entry = get_entry_from_path(ark, milo_path)?;

    let data = ark.get_stream(entry.id).ok()?;

    let mut stream = MemoryStream::from_slice_as_read(&data);
    let milo = MiloArchive::from_stream(&mut stream).ok()?;

    let milo_path = std::path::Path::new(&entry.path);
    let system_info = SystemInfo::guess_system_info(&milo, &milo_path);

    let mut obj_dir = milo.unpack_directory(&system_info).ok()?;
    obj_dir.unpack_entries(&system_info).ok()?;

    Some(obj_dir)
}

fn get_entry_from_path<'a>(ark: &'a Ark, path: &str) -> Option<&'a ArkOffsetEntry> {
    let possible_paths = [
        path.to_owned(),
        get_path_with_gen_folder(path),
    ];

    /*for p in possible_paths.iter() {
        log::debug!("{p}");
    }*/

    ark
        .entries
        .iter()
        .filter(|e| possible_paths.iter().any(|p| e.path.starts_with(p)))
        .next()
}

fn get_path_with_gen_folder(path: &str) -> String {
    let slash_idx = path.rfind('/');

    let Some(i) = slash_idx else {
        return format!("gen/{path}");
    };

    let (s1, s2) = path.split_at(i);
    format!("{s1}/gen{s2}")
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

// TODO: Load textures async