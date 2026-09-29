//! Display 配置分组。

use crate::State;
use crate::ui::grid::render_grid_ui;

/// 绘制显示相关配置：目前只有渲染窗口的清屏颜色。
pub fn render_display_ui(state: &mut State, ui: &mut egui::Ui) {
    render_grid_ui("display_grid", ui, |ui| {
        ui.heading("🎨 Display");
        ui.end_row();

        ui.label("Background Color:");
        let mut color = [
            (state.background_color.r * 255.0 + 0.5) as u8,
            (state.background_color.g * 255.0 + 0.5) as u8,
            (state.background_color.b * 255.0 + 0.5) as u8,
        ];
        if ui.color_edit_button_srgb(&mut color).changed() {
            state.background_color.r = color[0] as f64 / 255.;
            state.background_color.g = color[1] as f64 / 255.;
            state.background_color.b = color[2] as f64 / 255.;
        }
        ui.end_row();
    });
}
