//! Text Style 配置分组。

use crate::State;
use crate::defaults::{shadow_default, stroke_default};
use crate::ui::color_picker;
use crate::ui::grid::render_grid_ui;
use crate::ui::switch::toggle;

/// 绘制正文样式：字号、字重、填充色，以及描边与阴影的开关和参数。
///
/// 开关关闭时把对应样式置为 `None`；重新打开时用默认值填充，避免留下上一次调过的参数。
pub fn render_text_style_ui(state: &mut State, ui: &mut egui::Ui) {
    render_grid_ui("style_grid", ui, |ui| {
        ui.heading("📄 Text Style");
        ui.end_row();

        ui.label("Font Size:");
        ui.add(
            egui::DragValue::new(&mut state.text_config.font_size)
                .speed(1.0)
                .range(8.0..=128.0),
        );
        ui.end_row();

        ui.label("Font Weight:");
        ui.add(egui::Slider::new(
            &mut state.text_config.font_weight,
            250..=900,
        ));
        ui.end_row();

        ui.label("Fill Color:");
        if color_picker::color_picker_srgba(ui, &mut state.text_config.fill_color).changed() {
            state.config_changed = true;
        }
        ui.end_row();

        render_stroke_rows(state, ui);
        render_shadow_rows(state, ui);
    });
}

fn render_stroke_rows(state: &mut State, ui: &mut egui::Ui) {
    ui.label("Enable Stroke");
    ui.add(toggle(&mut state.stroke_enabled));
    ui.end_row();

    if !state.stroke_enabled {
        state.text_config.stroke = None;
        return;
    }
    if state.text_config.stroke.is_none() {
        state.text_config.stroke = Some(stroke_default());
    }

    if let Some(stroke) = &mut state.text_config.stroke {
        ui.label("    Width:");
        ui.add(
            egui::DragValue::new(&mut stroke.stroke_width)
                .speed(0.1)
                .range(0.0..=30.0),
        );
        ui.end_row();

        ui.label("    Color:");
        color_picker::color_picker_srgba(ui, &mut stroke.stroke_color);
        ui.end_row();
    }
}

fn render_shadow_rows(state: &mut State, ui: &mut egui::Ui) {
    ui.label("Enable Shadow");
    ui.add(toggle(&mut state.shadow_enabled));
    ui.end_row();

    if !state.shadow_enabled {
        state.text_config.shadow = None;
        return;
    }
    if state.text_config.shadow.is_none() {
        state.text_config.shadow = Some(shadow_default());
    }

    if let Some(shadow) = &mut state.text_config.shadow {
        ui.label("    Offset X:");
        ui.add(
            egui::DragValue::new(&mut shadow.shadow_offset_x)
                .speed(0.5)
                .range(-50.0..=50.0),
        );
        ui.end_row();

        ui.label("    Offset Y:");
        ui.add(
            egui::DragValue::new(&mut shadow.shadow_offset_y)
                .speed(0.5)
                .range(-50.0..=50.0),
        );
        ui.end_row();

        ui.label("    Blur:");
        ui.add(
            egui::DragValue::new(&mut shadow.shadow_blur)
                .speed(0.5)
                .range(0.0..=100.0),
        );
        ui.end_row();

        ui.label("    Width:");
        ui.add(
            egui::DragValue::new(&mut shadow.shadow_width)
                .speed(0.1)
                .range(0.0..=20.0),
        );
        ui.end_row();

        ui.label("    Color:");
        color_picker::color_picker_srgba(ui, &mut shadow.shadow_color);
        ui.end_row();
    }
}
