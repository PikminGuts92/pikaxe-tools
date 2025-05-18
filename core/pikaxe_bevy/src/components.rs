use bevy::prelude::*;
use bevy::tasks::Task;

pub(crate) enum TextureType {
    Diffuse,
    Normal,
    Emissive
}

#[derive(Component)]
pub(crate) struct MiloAsyncTexture {
    pub tex_name: String,
    pub image_task: Task<Image>,
    pub mat_handles: Vec<(Handle<StandardMaterial>, TextureType)>,
}

#[derive(Component)]
pub struct MiloBandPlacer;

#[derive(Component)]
pub struct MiloObject {
    pub id: u32,
    pub name: String,
    pub dir: String,
}

#[derive(Component)]
pub struct ParentOverride;

#[derive(Component)]
pub struct MiloMesh {
    pub verts: usize,
    pub faces: usize,
}

#[derive(Component)]
pub struct MiloBone;

#[derive(Component)]
pub struct MiloCam;

#[derive(Component)]
pub struct MiloCharHair;

#[derive(Component)]
pub struct MiloCharClip(pub Handle<AnimationClip>);

#[derive(Component)]
pub struct MiloGroup {
    pub objects: Vec<u32>,
}

#[derive(Component)]
pub struct MiloRoot;

#[derive(Component)]
pub struct CloneTransform(pub Entity);