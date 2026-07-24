use egui::{Color32, RichText, TextWrapMode};

use super::DebugSnapshot;

pub(super) fn draw(context: &egui::Context, snapshot: DebugSnapshot) {
    egui::Area::new("f3_debug_overlay".into())
        .fixed_pos(egui::pos2(10.0, 10.0))
        .interactable(false)
        .show(context, |ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
            ui.visuals_mut().override_text_color = Some(Color32::WHITE);
            ui.label(RichText::new(format!("FPS: {:.0}", snapshot.fps)).color(Color32::WHITE));
            ui.label(
                RichText::new(format!(
                    "XYZ: {:.2} / {:.2} / {:.2}",
                    snapshot.player_position.x,
                    snapshot.player_position.y,
                    snapshot.player_position.z
                ))
                .color(Color32::WHITE),
            );
            ui.label(
                RichText::new(format!(
                    "Chunk: {} / {} / {}",
                    snapshot.player_chunk.x, snapshot.player_chunk.y, snapshot.player_chunk.z
                ))
                .color(Color32::WHITE),
            );
            ui.label(
                RichText::new(format!(
                    "Render distance: {} chunks",
                    snapshot.render_distance
                ))
                .color(Color32::WHITE),
            );
            ui.label(
                RichText::new(format!("Chunks loaded: {}", snapshot.loaded_chunks))
                    .color(Color32::WHITE),
            );
        });
}
