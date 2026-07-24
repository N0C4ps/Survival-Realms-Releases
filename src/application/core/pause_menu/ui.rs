use egui::{Align2, Color32, FontId, Rect, Sense, Stroke, StrokeKind, Vec2};

use super::{PauseMenuAction, PauseMenuState, state::PauseMenuPage};
use crate::application::core::settings::GameSettings;

const BASE_BUTTON_SIZE: Vec2 = Vec2::new(300.0, 40.0);

pub(crate) fn draw(
    context: &egui::Context,
    state: &mut PauseMenuState,
    settings: &mut GameSettings,
) -> Option<PauseMenuAction> {
    let mut action = None;
    let scale = interface_scale(context);
    draw_header(context, state.page, scale);
    egui::Area::new("pause_menu".into())
        .anchor(Align2::LEFT_CENTER, egui::vec2(72.0 * scale, 0.0))
        .show(context, |ui| {
            ui.set_width(BASE_BUTTON_SIZE.x * scale);
            match state.page {
                PauseMenuPage::Main => {
                    let resume = menu_button(ui, "Resume", state.selected == 0, scale);
                    if resume.hovered() {
                        state.selected = 0;
                    }
                    if resume.clicked() {
                        action = Some(PauseMenuAction::Resume);
                    }
                    ui.add_space(7.0 * scale);
                    let save = menu_button(ui, "Save", state.selected == 1, scale);
                    if save.hovered() {
                        state.selected = 1;
                    }
                    if save.clicked() {
                        action = Some(PauseMenuAction::Save);
                    }
                    ui.add_space(34.0 * scale);
                    let options = menu_button(ui, "Options", state.selected == 2, scale);
                    if options.hovered() {
                        state.selected = 2;
                    }
                    if options.clicked() {
                        state.page = PauseMenuPage::Options;
                        state.selected = 0;
                    }
                    ui.add_space(7.0 * scale);
                    let quit = menu_button(ui, "Quit", state.selected == 3, scale);
                    if quit.hovered() {
                        state.selected = 3;
                    }
                    if quit.clicked() {
                        action = Some(PauseMenuAction::Quit);
                    }
                }
                PauseMenuPage::Options => {
                    let graphics = menu_button(ui, "Graphics", state.selected == 0, scale);
                    if graphics.hovered() {
                        state.selected = 0;
                    }
                    if graphics.clicked() {
                        state.page = PauseMenuPage::Graphics;
                        state.selected = 0;
                    }
                    ui.add_space(7.0 * scale);
                    let controls =
                        menu_button(ui, "Mouse and Keyboard", state.selected == 1, scale);
                    if controls.hovered() {
                        state.selected = 1;
                    }
                    if controls.clicked() {
                        state.page = PauseMenuPage::MouseAndKeyboard;
                        state.selected = 0;
                    }
                    ui.add_space(34.0 * scale);
                    if menu_button(ui, "Back", state.selected == 2, scale).clicked() {
                        state.page = PauseMenuPage::Main;
                        state.selected = 2;
                    }
                }
                PauseMenuPage::Graphics => {
                    let mut render_distance = i32::from(settings.render_distance);
                    let mut brightness = i32::from(settings.brightness);
                    let changed =
                        setting_slider(ui, "Render Distance", &mut render_distance, 2, 12, scale)
                            | setting_slider(
                                ui,
                                "Brightness / Gamma",
                                &mut brightness,
                                1,
                                100,
                                scale,
                            );
                    if changed {
                        settings.render_distance = render_distance as u8;
                        settings.brightness = brightness as u8;
                        action = Some(PauseMenuAction::SettingsChanged(*settings));
                    }
                    ui.add_space(30.0 * scale);
                    if menu_button(ui, "Back", true, scale).clicked() {
                        state.page = PauseMenuPage::Options;
                        state.selected = 0;
                    }
                }
                PauseMenuPage::MouseAndKeyboard => {
                    let mut fov = i32::from(settings.fov);
                    let mut sensitivity = i32::from(settings.mouse_sensitivity);
                    let changed = setting_slider(ui, "FOV", &mut fov, 0, 200, scale)
                        | setting_slider(ui, "Mouse Sensitivity", &mut sensitivity, 0, 100, scale);
                    if changed {
                        settings.fov = fov as u16;
                        settings.mouse_sensitivity = sensitivity as u8;
                        action = Some(PauseMenuAction::SettingsChanged(*settings));
                    }
                    ui.add_space(30.0 * scale);
                    if menu_button(ui, "Back", true, scale).clicked() {
                        state.page = PauseMenuPage::Options;
                        state.selected = 1;
                    }
                }
            }
        });
    action
}

fn setting_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut i32,
    minimum: i32,
    maximum: i32,
    scale: f32,
) -> bool {
    let size = Vec2::new(BASE_BUTTON_SIZE.x * scale, 62.0 * scale);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let mut changed = false;
    if (response.dragged() || response.clicked())
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let amount = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        let new_value = (minimum as f32 + amount * (maximum - minimum) as f32).round() as i32;
        let new_value = new_value.clamp(minimum, maximum);
        changed = *value != new_value;
        *value = new_value;
    }

    let painter = ui.painter();
    let font = FontId::proportional(16.0 * scale);
    painter.text(
        rect.left_top(),
        Align2::LEFT_TOP,
        label,
        font.clone(),
        Color32::WHITE,
    );
    painter.text(
        rect.right_top(),
        Align2::RIGHT_TOP,
        value.to_string(),
        font,
        Color32::WHITE,
    );

    let track_y = rect.bottom() - 12.0 * scale;
    let track = Rect::from_min_max(
        egui::pos2(rect.left(), track_y - 2.0 * scale),
        egui::pos2(rect.right(), track_y + 2.0 * scale),
    );
    painter.rect_filled(track, 0, Color32::from_gray(48));
    let amount = (*value - minimum) as f32 / (maximum - minimum) as f32;
    let filled = Rect::from_min_max(
        track.left_top(),
        egui::pos2(track.left() + track.width() * amount, track.bottom()),
    );
    painter.rect_filled(filled, 0, Color32::from_gray(205));
    let knob = egui::pos2(track.left() + track.width() * amount, track.center().y);
    painter.circle_filled(knob, 6.0 * scale, Color32::WHITE);
    ui.add_space(8.0 * scale);
    changed
}

fn interface_scale(context: &egui::Context) -> f32 {
    let screen = context.content_rect();
    interface_scale_for_size(screen.size())
}

fn interface_scale_for_size(size: Vec2) -> f32 {
    (size.x / 1280.0).min(size.y / 720.0).clamp(1.0, 2.0)
}

fn draw_header(context: &egui::Context, page: PauseMenuPage, scale: f32) {
    let screen = context.content_rect();
    egui::Area::new("pause_menu_header".into())
        .fixed_pos(screen.left_top())
        .interactable(false)
        .show(context, |ui| {
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(screen.width(), 64.0 * scale), Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(rect, 0, Color32::from_rgba_unmultiplied(3, 5, 4, 188));
            painter.line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                Stroke::new(1.0, Color32::from_white_alpha(38)),
            );
            painter.line_segment(
                [
                    rect.left_bottom() + egui::vec2(0.0, 2.0),
                    rect.right_bottom() + egui::vec2(0.0, 2.0),
                ],
                Stroke::new(2.0, Color32::from_white_alpha(10)),
            );
            let title = match page {
                PauseMenuPage::Main => "Paused",
                PauseMenuPage::Options => "Options",
                PauseMenuPage::Graphics => "Graphics",
                PauseMenuPage::MouseAndKeyboard => "Mouse and Keyboard",
            };
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                title,
                FontId::proportional(22.0 * scale),
                Color32::WHITE,
            );
        });
}

fn menu_button(ui: &mut egui::Ui, text: &str, selected: bool, scale: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(BASE_BUTTON_SIZE * scale, Sense::click());
    let selected = selected || response.hovered() || response.has_focus();
    let painter = ui.painter();
    soft_edge(painter, rect);
    painter.rect_filled(
        rect,
        0,
        if selected {
            Color32::from_rgba_unmultiplied(12, 13, 11, 232)
        } else {
            Color32::from_rgba_unmultiplied(5, 6, 5, 218)
        },
    );
    if selected {
        glow(painter, rect);
        painter.rect_stroke(
            rect,
            0,
            Stroke::new(2.0, Color32::WHITE),
            StrokeKind::Inside,
        );
    } else {
        painter.rect_stroke(
            rect,
            0,
            Stroke::new(1.0, Color32::from_gray(55)),
            StrokeKind::Inside,
        );
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(17.0 * scale),
        Color32::WHITE,
    );
    response
}

fn soft_edge(painter: &egui::Painter, rect: Rect) {
    painter.rect_stroke(
        rect.expand(1.0),
        0,
        Stroke::new(2.0, Color32::from_white_alpha(30)),
        StrokeKind::Middle,
    );
    painter.rect_stroke(
        rect.expand(3.0),
        0,
        Stroke::new(2.0, Color32::from_white_alpha(14)),
        StrokeKind::Middle,
    );
    painter.rect_stroke(
        rect.expand(5.0),
        0,
        Stroke::new(2.0, Color32::from_white_alpha(6)),
        StrokeKind::Middle,
    );
}

fn glow(painter: &egui::Painter, rect: Rect) {
    painter.rect_stroke(
        rect.expand(2.0),
        0,
        Stroke::new(2.0, Color32::from_white_alpha(85)),
        StrokeKind::Middle,
    );
    painter.rect_stroke(
        rect.expand(4.0),
        0,
        Stroke::new(2.0, Color32::from_white_alpha(28)),
        StrokeKind::Middle,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reopening_the_pause_menu_returns_to_the_main_page() {
        let mut state = PauseMenuState {
            page: PauseMenuPage::Options,
            selected: 3,
        };
        state.reset();
        assert_eq!(state.page, PauseMenuPage::Main);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn interface_grows_on_large_viewports_without_unbounded_scaling() {
        assert_eq!(interface_scale_for_size(Vec2::new(1280.0, 720.0)), 1.0);
        assert_eq!(interface_scale_for_size(Vec2::new(1920.0, 1080.0)), 1.5);
        assert_eq!(interface_scale_for_size(Vec2::new(3840.0, 2160.0)), 2.0);
    }
}
