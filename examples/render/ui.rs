mod color_picker;
mod display;
mod fonts;
mod grid;
mod layout;
mod progress;
mod switch;
mod text_style;

use egui::FullOutput;
use winit::window::Window;

use crate::State;
use crate::ui::display::render_display_ui;
use crate::ui::fonts::render_fonts_ui;
use crate::ui::layout::render_layout_ui;
use crate::ui::progress::render_progress_ui;
use crate::ui::text_style::render_text_style_ui;

/// 绘制左上角的交互提示浮层。
fn render_interaction_notice(state: &mut State, ui: &mut egui::Ui) {
    let Some(notice) = state.interaction_notice.clone() else {
        return;
    };
    egui::Window::new("交互提示")
        .anchor(egui::Align2::CENTER_TOP, [0.0, 16.0])
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label(notice);
            if ui.button("关闭").clicked() {
                state.interaction_notice = None;
            }
        });
}

/// 绘制左侧的多行富文本输入框。
///
/// 输入文本远长于面板高度，因此外面套一层滚动区：输入框自己会随内容长高，滚动区把它夹在面板的
/// 剩余高度内，底部面板不会被撑开去盖住渲染区域。
fn render_text_input(state: &mut State, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.heading("Text");
        let height = ui.text_style_height(&egui::TextStyle::Name("custom_font".into())) * 21.0;
        egui::ScrollArea::vertical()
            .id_salt("text_input_scroll")
            .max_height(height)
            .min_scrolled_height(height)
            .auto_shrink([true, false])
            .show(ui, |ui| {
                egui::TextEdit::multiline(&mut state.input_text)
                    .desired_width(500.)
                    .desired_rows(16)
                    .font(egui::TextStyle::Name("custom_font".into()))
                    .show(ui);
            });
    });
}

pub fn render_control_panel_ui(state: &mut State, window: &Window) -> FullOutput {
    let raw_input = state.egui_state.take_egui_input(window);
    // 先取出上下文：`egui::Context` 内部是共享句柄，克隆很轻，且能避免它借用整个 `state`，
    // 从而允许回调里修改其他字段。
    let context = state.egui_context.clone();
    context.run_ui(raw_input, |ui| {
        render_interaction_notice(state, ui);

        // 底部面板：左侧文本输入，右侧配置分组。
        egui::Panel::bottom("text_input_panel")
            .resizable(true)
            .default_size(360.0)
            .show(ui, |ui| {
                ui.add_space(6.);

                ui.vertical(|ui| {
                    // 逐字进度：配置面板最上方一条占满宽度的滑条，按 `glyphs` 结束下标推进。
                    render_progress_ui(state, ui);
                    ui.horizontal(|ui| {
                        render_text_input(state, ui);
                        ui.separator();
                        ui.vertical(|ui| {
                            render_display_ui(state, ui);
                            render_fonts_ui(state, ui);
                            ui.add_space(10.);
                            render_layout_ui(state, ui);
                        });
                        ui.separator();
                        render_text_style_ui(state, ui);
                    });
                });
                ui.add_space(6.);
            });
    })
}
