use bevy::prelude::*;
use pikaxe::ark::Ark;

#[derive(Default, Resource)]
pub struct MiloState {
    pub ark: Option<Ark>,
}

// TODO: Track object hierarchy somehow (object id node tree?)