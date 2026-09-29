//! Layout 配置分组。

use huozi::layout::ParagraphAlignment;

use crate::State;
use crate::ui::grid::render_grid_ui;

/// 绘制段落布局约束：宽高、行高倍率、首行缩进与末行对齐。
pub fn render_layout_ui(state: &mut State, ui: &mut egui::Ui) {
    render_grid_ui("layout_grid", ui, |ui| {
        ui.heading("⚙ Layout");
        ui.end_row();

        ui.label("Box Width:");
        ui.add(egui::Slider::new(
            state.layout_config.box_width.get_or_insert(1280.0),
            0.0..=1280.0,
        ));
        ui.end_row();

        ui.label("Box Height:");
        ui.add(egui::Slider::new(
            state.layout_config.box_height.get_or_insert(360.0),
            0.0..=1000.0,
        ));
        ui.end_row();

        ui.label("Line Height:");
        ui.add(
            egui::DragValue::new(&mut state.layout_config.line_height)
                .speed(0.1)
                .range(0.5..=3.0),
        );
        ui.end_row();

        ui.label("Indent:");
        ui.add(
            egui::DragValue::new(&mut state.layout_config.indent)
                .speed(1.0)
                .range(0.0..=200.0),
        );
        ui.end_row();

        ui.label("Align:");
        egui::ComboBox::from_id_salt("paragraph_align")
            .selected_text(align_label(state.layout_config.align))
            .show_ui(ui, |ui| {
                for (alignment, label) in [
                    (ParagraphAlignment::Start, "Start"),
                    (ParagraphAlignment::Center, "Center"),
                    (ParagraphAlignment::End, "End"),
                ] {
                    ui.selectable_value(&mut state.layout_config.align, alignment, label);
                }
            });
        ui.end_row();
    });
}

fn align_label(alignment: ParagraphAlignment) -> &'static str {
    match alignment {
        ParagraphAlignment::Start => "Start",
        ParagraphAlignment::Center => "Center",
        ParagraphAlignment::End => "End",
    }
}
