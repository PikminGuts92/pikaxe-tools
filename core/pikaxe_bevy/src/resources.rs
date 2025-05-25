use bevy::prelude::*;
use pikaxe::SystemInfo;
use pikaxe::ark::{Ark, ArkOffsetEntry};
use pikaxe::io::*;
use pikaxe::scene::Object;
use pikaxe::scene::ObjectDir;
use std::collections::HashMap;

#[derive(Default, Resource)]
pub struct MiloState {
    pub ark: Option<Ark>,
    pub objects: Vec<Object>,
}

impl MiloState {
    pub fn open_milo(&self, milo_path: &str) -> Option<(SystemInfo, ObjectDir)> {
        let ark = self.ark.as_ref()?;

        let entry = get_entry_from_path(ark, milo_path)?;

        let data = ark.get_stream(entry.id).ok()?;

        let mut stream = MemoryStream::from_slice_as_read(&data);
        let milo = MiloArchive::from_stream(&mut stream).ok()?;

        let milo_path = std::path::Path::new(&entry.path);
        let system_info = SystemInfo::guess_system_info(&milo, &milo_path);

        let mut obj_dir = milo.unpack_directory(&system_info).ok()?;
        obj_dir.unpack_entries(&system_info).ok()?;

        Some((system_info, obj_dir))
    }
}

#[derive(Default, Resource)]
pub struct MiloEntityMap(pub HashMap<String, Entity>);

impl MiloEntityMap {
    pub fn get_entity(&self, object_name: &str) -> Option<Entity> {
        self.0.get(object_name).map(|e| *e)
    }

    pub fn set_entity(&mut self, object_name: &str, entity: Entity) {
        if let Some(obj_entity) = self.0.get_mut(object_name) {
            *obj_entity = entity;
        } else {
            self.0.insert(object_name.to_owned(), entity);
        }
    }

    pub fn remove(&mut self, object_name: &str) {
        self.0.remove(object_name);
    }
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

// TODO: Track object hierarchy somehow (object id node tree?)