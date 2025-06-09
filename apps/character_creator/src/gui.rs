// TODO: Move to mod folder?

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{egui, EguiContext, EguiContextPass, EguiGlobalSettings, EguiPlugin};
use crate::components::*;
use crate::resources::*;

pub struct GuiPlugin;

#[derive(Component, Default)]
struct EntitiesSortedByName(Vec<(Entity, String)>, bool);

impl EntitiesSortedByName {
    fn add(&mut self, value: (Entity, String)) {
        self.0.push(value);
        self.1 = false;
    }

    fn sort(&mut self) {
        if !self.1 {
            self.0.sort_by(|(_, a), (_, b)| a.cmp(b));
            self.1 = true;
        }
    }

    fn entries(&self) -> &Vec<(Entity, String)> {
        &self.0
    }
}

#[derive(Resource, Default)]
struct SortedNames {
    anims: EntitiesSortedByName,
}

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(EguiPlugin {
                enable_multipass_for_primary_context: true
            })
            .insert_resource(SortedNames::default())
            .add_systems(Startup, init_gui)
            .add_systems(EguiContextPass, (
                render_toolbar,
                render_window,
            ).chain())
            .add_systems(Update, display_name_entities_added);
    }
}

fn display_name_entities_added(
    mut sorted_names: ResMut<SortedNames>,
    add_anim_graph_query: Query<(Entity, &GuiDisplayName), (With<CharacterAnimations>, Added<GuiDisplayName>)>,
) {
    for (anim_entity, GuiDisplayName(display_name)) in add_anim_graph_query.iter() {
        sorted_names.anims.add((anim_entity, display_name.to_owned()));
    }

    sorted_names.anims.sort();
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

    mut selected_animation: ResMut<SelectedAnimation>,
    selected_animation_options: Res<SelectedAnimationOptions>,
) {
    let ctx = egui_ctx_query.into_inner().get();

    egui::TopBottomPanel::bottom("panel_bottom")
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Character:").on_hover_text("Choose selected character");

                egui::ComboBox::from_id_salt("cb_sel_character")
                    .selected_text(selected_character.0
                        .and_then(|s| selected_character_options.0.get(s)).map(|(_, d, _)| d.as_str()).unwrap_or("(None)"))
                    .show_ui(ui, |ui| {
                        // TODO: Refactor this to not rely on copy
                        let mut selected_character_copy = selected_character.0.clone();

                        for (i, (_short_name, display_name, _)) in selected_character_options.0.iter().enumerate() {
                            if ui.selectable_value(&mut selected_character_copy, Some(i), display_name).changed() {
                                selected_character.0 = selected_character_copy;
                            };
                        }
                    });

                ui.label("Animation:").on_hover_text("Choose selected animation");

                egui::ComboBox::from_id_salt("cb_sel_animation")
                    .selected_text(selected_animation.0
                        .and_then(|s| selected_animation_options.0.get(s)).map(|(_, d)| d.as_str()).unwrap_or("(None)"))
                    .show_ui(ui, |ui| {
                        // TODO: Refactor this to not rely on copy
                        let mut selected_animation_copy = selected_animation.0.clone();

                        for (i, (_short_name, display_name)) in selected_animation_options.0.iter().enumerate() {
                            if ui.selectable_value(&mut selected_animation_copy, Some(i), display_name).changed() {
                                selected_animation.0 = selected_animation_copy;
                            };
                        }
                    });
            });
        });
}


fn render_window(
    egui_ctx_query: Single<&EguiContext, With<PrimaryWindow>>,
) {
    let ctx = egui_ctx_query.into_inner().get();

    egui::Window::new("Some Menu")
        .show(ctx, |ui| {
            ui.label("Hello test");
        });
}

fn update_selectors(
    egui_ctx_query: Single<&EguiContext, With<PrimaryWindow>>,
) {
    let ctx = egui_ctx_query.into_inner().get();

    //ctx.ui
}