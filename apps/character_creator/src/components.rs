use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;


#[derive(Component, Default)]
#[component(immutable)]
pub struct GuiDisplayName(pub String);

#[derive(Component, Default)]
pub struct CharacterAnimations {
    pub enter_clip: Option<(Handle<AnimationClip>, Handle<AnimationGraph>)>,
    pub loop_clip: Option<(Handle<AnimationClip>, Handle<AnimationGraph>)>,
    pub enter_played: bool,
}