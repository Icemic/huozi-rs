use csscolorparser::Color;
use egui::color_picker::show_color_at;
use egui::{
    Color32, Popup, PopupCloseBehavior, Response, Sense, StrokeKind, Ui, WidgetInfo, WidgetType,
};

/// A color picker button that shows a popup color picker when clicked.
///
/// The color is represented in sRGBA space **without** premultiplied alpha.
pub fn color_picker_srgba(ui: &mut Ui, color: &mut Color) -> Response {
    let popup_id = ui.auto_id_with("popup");
    let open = Popup::is_id_open(ui.ctx(), popup_id);

    let color_u8 = color.to_rgba8();
    let mut color32 =
        Color32::from_rgba_unmultiplied_const(color_u8[0], color_u8[1], color_u8[2], color_u8[3]);

    let mut button_response = color_button(ui, color32, open);
    if ui.style().explanation_tooltips {
        button_response = button_response.on_hover_text("Click to edit color");
    }

    const COLOR_SLIDER_WIDTH: f32 = 275.0;

    Popup::menu(&button_response)
        .id(popup_id)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.spacing_mut().slider_width = COLOR_SLIDER_WIDTH;

            if egui::widgets::color_picker::color_picker_color32(
                ui,
                &mut color32,
                egui::color_picker::Alpha::OnlyBlend,
            ) {
                *color = color32.to_srgba_unmultiplied().into();
                button_response.mark_changed();
            }
        });

    button_response
}

fn color_button(ui: &mut Ui, color: Color32, open: bool) -> Response {
    let size = ui.spacing().interact_size;
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::new(WidgetType::ColorButton));

    if ui.is_rect_visible(rect) {
        let visuals = if open {
            &ui.visuals().widgets.open
        } else {
            ui.style().interact(&response)
        };
        let rect = rect.expand(visuals.expansion);

        let stroke_width = 1.0;
        show_color_at(ui.painter(), color, rect.shrink(stroke_width));

        let corner_radius = visuals.corner_radius.at_most(2); // Can't do more rounding because the background grid doesn't do any rounding
        ui.painter().rect_stroke(
            rect,
            corner_radius,
            (stroke_width, visuals.bg_fill), // Using fill for stroke is intentional, because default style has no border
            StrokeKind::Inside,
        );
    }

    response
}
