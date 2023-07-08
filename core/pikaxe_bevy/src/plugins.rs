use crate::prelude::*;
use bevy::prelude::*;
use pikaxe::ark::{Ark, ArkOffsetEntry};
use pikaxe::io::*;
use pikaxe::scene::ObjectDir;
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
                .map(|p| Ark::from_path(p).expect("Can't open ark file"))
        };

        app.add_event::<ClearMiloScene>();
        app.add_event::<LoadMiloScene>();

        app.insert_resource(state);

        app.add_system(process_milo_scene_events);
    }
}

// TODO: Move to separate file?
fn process_milo_scene_events(
    mut scene_events_reader: EventReader<LoadMiloScene>,
    state: Res<MiloState>,
) {
    /*for e in state.ark.as_ref().unwrap().entries.iter() {
        log::debug!("{}", &e.path);
    }*/

    let ark = state.ark.as_ref().unwrap();

    // TODO: Check if path ends in .milo
    for LoadMiloScene(milo_path) in scene_events_reader.iter() {
        log::debug!("Loading Scene: \"{}\"", milo_path);

        let milo = open_milo(ark, &milo_path).unwrap();
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

// TODO: Load textures async