use bevy::prelude::*;
use pikaxe::ark::Ark;
use pikaxe::scene::Object;

#[derive(Default, Resource)]
pub struct MiloState {
    pub ark: Option<Ark>,
    pub objects: Vec<Object>,
}

// TODO: Track object hierarchy somehow (object id node tree?)