// TODO: Move to mod folder?

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{egui, EguiContext, EguiContextPass, EguiGlobalSettings, EguiPlugin};
use crate::resources::*;

pub struct GuiPlugin;

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true
        });

        app.add_systems(Startup, init_gui);
        app.add_systems(EguiContextPass, render_toolbar);
    }
}

fn init_gui(
    world: &mut World,
) {
    let mut egui_settings = world.get_resource_mut::<EguiGlobalSettings>().unwrap();

    egui_settings.enable_absorb_bevy_input_system = true;
}

fn render_toolbar(
    egui_ctx_query: Single<&EguiContext, With<PrimaryWindow>>,
    mut selected_character: ResMut<SelectedCharacter>,
    selected_character_options: Res<SelectedCharacterOptions>,
) {
    let ctx = egui_ctx_query.into_inner().get();

    egui::TopBottomPanel::bottom("panel_bottom")
        .show(ctx, |ui| {
            //egui::Label::new("")
            ui.horizontal(|ui| {
                ui.label("Character:").on_hover_text("Choose selected character");

                egui::ComboBox::from_id_salt("cb_sel_character")
                    .selected_text(selected_character.0
                        .and_then(|s| selected_character_options.0.get(s)).map(|(_, d)| d.as_str()).unwrap_or("(None)"))
                    .show_ui(ui, |ui| {
                        //ui.selectable_value(&mut character, "alterna1", "Judy Nails");
                        //ui.selectable_value(&mut character, "grim", "Grim Ripper");

                        for (i, (_short_name, display_name)) in selected_character_options.0.iter().enumerate() {
                            ui.selectable_value(&mut selected_character.0, Some(i), display_name);
                        }
                    });
            });
        });
}