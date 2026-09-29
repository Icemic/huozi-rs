//! 逐字显示进度控件。

use crate::State;

/// 绘制逐字显示进度：配置面板最上方一条占满宽度的滑条。
///
/// 滑条的值是 `RichTextLayoutOutput.glyphs` 的结束下标：拖到最左端什么都不显示，拖到最右端显示
/// 全部（此时 `progress` 为 `None`）。它只改变可见前缀，不触发重新排版。
///
/// 滑条宽度写在局部 `scope` 里：`spacing_mut` 会沿 Ui 树向下继承，写在外层会让后面所有列的滑条
/// 都跟着取满面板宽度，把配置列挤出屏幕。
pub fn render_progress_ui(state: &mut State, ui: &mut egui::Ui) {
    let total = state.element_count;
    let mut end = state.progress.unwrap_or(total).min(total);
    ui.label(format!("逐字进度 {end} / {total}"));

    let response = ui.scope(|ui| {
        ui.spacing_mut().slider_width = ui.available_width();
        ui.add(
            egui::Slider::new(&mut end, 0..=total)
                .show_value(false)
                .clamping(egui::SliderClamping::Always),
        )
    });
    if response.inner.changed() {
        // 拖到最右端表示显示全部，不保留多余的进度值。
        state.progress = (end < total).then_some(end);
        state.progress_changed = true;
    }
}
