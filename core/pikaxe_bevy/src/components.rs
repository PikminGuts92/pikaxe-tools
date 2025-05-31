use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::tasks::Task;

use crate::prelude::MiloEntityMap;

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
pub struct CustomParent(pub String);

#[derive(Component)]
//#[component(on_add = add_milo_object)]
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

#[derive(Component)]
pub struct CloneWorldTransform(pub Entity);

#[derive(Component)]
pub struct PhysicsControlledBone;

#[derive(Component)]
pub struct OriginalTransform(pub Transform);

/*fn add_milo_object(
    mut world: DeferredWorld,
    HookContext { entity, .. }: HookContext,
    //trigger: Trigger<OnInsert, MiloObject>,
    //milo_object_query: Query<&MiloObject>,
    //mut milo_entity_map: ResMut<MiloEntityMap>,
) {
    //let milo_object = milo_object_query.get(trigger.target()).unwrap(); // Observers with results not supported
    //milo_entity_map.set_entity(&milo_object.name, trigger.target());

    let milo_object = world.get::<MiloObject>(entity).unwrap();
    let mut milo_entity_map = world.get_resource_mut::<MiloEntityMap>().unwrap();

    milo_entity_map.set_entity(&milo_object.name, entity);
}*/