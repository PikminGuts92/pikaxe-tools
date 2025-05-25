use bevy::prelude::*;
use bevy::ecs::system::EntityCommands;

#[derive(Event)]
pub struct ClearMiloScene;

#[derive(Event)]
pub struct LoadMiloScene(pub String);

//#[derive(Event)]
////pub struct LoadMiloSceneWithComponents(pub String, for<'a> fn(&'a mut EntityCommands) -> &'a mut EntityCommands);
//pub struct LoadMiloSceneWithCommands<F>(pub String, pub F)
//    where for<'a, 'x, 'y, 'z> F: Fn(&'a mut EntityCommands<'x, 'y, 'z>) -> &'a mut EntityCommands<'x, 'y, 'z>;
//pub struct LoadMiloSceneWithCommands<'a, 'x, 'y, 'z>(pub String, pub impl Fn(&'a mut EntityCommands<'x, 'y, 'z>) -> &'a mut EntityCommands<'x, 'y, 'z>);

#[derive(Event)]
//pub struct LoadMiloSceneWithComponent<F>(pub String, pub F)
//    where F: Fn(&Object) -> Option<Box<dyn Component<Storage = SparseStorage>>>;
//pub struct LoadMiloSceneWithComponent<T: Bundle>(pub String, pub T);
pub struct LoadMiloSceneWithCommands(pub String, pub fn(&mut EntityCommands));

#[derive(Event)]
pub struct LoadMiloSceneComplete(pub String);

#[derive(Event)]
pub struct UpdateSkinnedMeshes(pub String);

#[derive(Event)]
pub struct SetCloneWorldTransform(pub Entity, pub String);