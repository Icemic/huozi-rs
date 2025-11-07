mod color_picker;
mod grid;
mod switch;

use egui::FullOutput;
use huozi::layout::LayoutDirection;
use winit::window::Window;

use crate::defaults::{shadow_default, stroke_default};
use crate::fonts::get_builtin_fonts;
use crate::ui::grid::render_grid_ui;
use crate::ui::switch::toggle;
use crate::State;

pub fn render_control_panel_ui(state: &mut State, window: &Window) -> FullOutput {
    let raw_input = state.egui_state.take_egui_input(window);
    state.egui_context.run(raw_input, |ctx| {
        // Bottom panel for text input and configuration
        egui::TopBottomPanel::bottom("text_input_panel")
            .resizable(false)
            .default_height(360.0)
            .show(ctx, |ui| {
                ui.add_space(6.);

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading("Text");
                        egui::TextEdit::multiline(&mut state.input_text)
                            .desired_width(500.)
                            .desired_rows(16)
                            .font(egui::TextStyle::Name("custom_font".into()))
                            .show(ui);
                    });
                    // ui.add_space(10.0);
                    ui.separator();
                    // Layout configuration

                    render_grid_ui("basic_grid", ui, |ui| {
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

                        ui.label("Font:");
                        egui::ComboBox::from_label("")
                            .selected_text(state.current_font.clone())
                            .show_ui(ui, |ui| {
                                for (font_name, _) in get_builtin_fonts().iter() {
                                    if ui
                                        .selectable_value(
                                            &mut state.current_font,
                                            font_name.to_string(),
                                            font_name.to_string(),
                                        )
                                        .changed()
                                    {
                                        let _ = state.huozi.take();
                                    }
                                }
                            });

                        ui.end_row();
                        // });

                        ui.add_space(10.);
                        ui.end_row();

                        ui.heading("⚙ Layout");
                        ui.end_row();

                        ui.label("Direction:").on_hover_text("Not supported by now");
                        ui.add_enabled_ui(false, |ui| {
                            ui.horizontal(|ui| {
                                ui.radio_value(
                                    &mut state.layout_config.direction,
                                    LayoutDirection::Horizontal,
                                    "Horizontal",
                                );
                                ui.radio_value(
                                    &mut state.layout_config.direction,
                                    LayoutDirection::Vertical,
                                    "Vertical",
                                );
                            });
                        });
                        ui.end_row();

                        ui.label("Box Width:");
                        ui.add(egui::Slider::new(
                            &mut state.layout_config.box_width,
                            0.0..=1280.0,
                        ));
                        ui.end_row();

                        ui.label("Box Height:");
                        ui.add(egui::Slider::new(
                            &mut state.layout_config.box_height,
                            0.0..=360.0,
                        ));
                        ui.end_row();

                        ui.label("Glyph Grid Size:");
                        ui.add(
                            egui::DragValue::new(&mut state.layout_config.glyph_grid_size)
                                .speed(1.0)
                                .range(8.0..=128.0),
                        );
                        ui.end_row();
                    });

                    ui.separator();

                    // Text style configuration
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

                        ui.label("Line Height:");
                        ui.add(
                            egui::DragValue::new(&mut state.text_config.line_height)
                                .speed(0.1)
                                .range(0.5..=3.0),
                        );
                        ui.end_row();

                        ui.label("Indent:");
                        ui.add(
                            egui::DragValue::new(&mut state.text_config.indent)
                                .speed(1.0)
                                .range(0.0..=200.0),
                        );
                        ui.end_row();

                        ui.label("Fill Color:");
                        if color_picker::color_picker_srgba(ui, &mut state.text_config.fill_color)
                            .changed()
                        {
                            state.config_changed = true;
                        }
                        ui.end_row();

                        // Stroke configuration
                        ui.label("Enable Stroke");
                        ui.add(toggle(&mut state.stroke_enabled));
                        ui.end_row();

                        if state.stroke_enabled {
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
                        } else {
                            state.text_config.stroke = None;
                        }

                        // Shadow configuration
                        ui.label("Enable Shadow");
                        ui.add(toggle(&mut state.shadow_enabled));
                        ui.end_row();

                        if state.shadow_enabled {
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
                        } else {
                            state.text_config.shadow = None;
                        }
                    });
                });
                ui.add_space(6.);
            });
    })
}
