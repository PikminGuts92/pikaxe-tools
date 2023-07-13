use bevy::prelude::*;

#[derive(Event)]
pub struct ClearMiloScene;

#[derive(Event)]
pub struct LoadMiloScene(pub String);

#[derive(Event)]
pub struct LoadMiloSceneComplete(pub String);

#[derive(Event)]
pub struct UpdateMiloObjectParents;