use bevy::prelude::*;

#[derive(Component)]
pub struct MiloObject {
    pub id: u32,
    pub name: String,
}

#[derive(Component)]
pub struct MiloMesh {
    pub verts: usize,
    pub faces: usize,
}

#[derive(Component)]
pub struct MiloBone;

#[derive(Component)]
pub struct MiloCharHair;

#[derive(Component)]
pub struct MiloGroup {
    pub objects: Vec<u32>,
}