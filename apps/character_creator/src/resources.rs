use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct SelectedCharacter(pub Option<usize>);

#[derive(Resource)]
pub struct SelectedCharacterOptions(pub Vec<(String, String)>); // shortname, display_name

impl Default for SelectedCharacterOptions {
    fn default() -> Self {
        Self(vec![
            ("alterna1".into(), "Judy Nails (Skulls)".into()),
            ("alterna2".into(), "Judy Nails (Snakes)".into())
        ])
    }
}

#[derive(Resource)]
pub struct SelectedAnimation {

}