use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct SelectedCharacter(pub Option<usize>);

#[derive(Resource)]
pub struct SelectedCharacterOptions(pub Vec<(String, String, bool)>); // shortname, display_name, is_guitarist

impl Default for SelectedCharacterOptions {
    fn default() -> Self {
        Self(vec![
            ("metal1".into(), "Axel Steel (Shirt)".into(), true),
            ("metal2".into(), "Axel Steel (Other Shirt)".into(), true),
            ("rock1".into(), "Casey Lynch (Skins)".into(), true),
            ("rock2".into(), "Casey Lynch (Shirts)".into(), true),
            ("classic".into(), "Clive Winston".into(), true),
            ("rockabill1".into(), "Eddie Knox (Reno)".into(), true),
            ("rockabill2".into(), "Eddie Knox (Vegas)".into(), true),
            ("grim".into(), "Grim".into(), true),
            ("glam1".into(), "Izzy Sparks (Codpiece)".into(), true),
            ("glam2".into(), "Izzy Sparks (Top Hat)".into(), true),
            ("punk1".into(), "Johnny Napalm (Mohawk)".into(), true),
            ("punk2".into(), "Johnny Napalm (Liberty Spikes)".into(), true),
            ("alterna1".into(), "Judy Nails (Skulls)".into(), true),
            ("alterna2".into(), "Judy Nails (Snakes)".into(), true),
            ("deathmetal1".into(), "Lars Ümlaüt (Gauntlets)".into(), true),
            ("deathmetal2".into(), "Lars Ümlaüt (Gargoyles)".into(), true),
            ("goth1".into(), "Pandora (Feathers)".into(), true),
            ("goth2".into(), "Pandora (Leathers)".into(), true),
            ("funk1".into(), "Xavier Stone".into(), true),
            ("metal_bass".into(), "Bassist".into(), false),
            ("metal_drummer".into(), "Drummer".into(), false),
            ("metal_singer".into(), "Singer".into(), false),
            ("female_singer".into(), "Female Singer".into(), false),
        ])
    }
}

#[derive(Resource)]
pub struct SelectedAnimation {

}