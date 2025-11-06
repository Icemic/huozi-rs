use egui::FullOutput;
use huozi::layout::{LayoutDirection, ShadowStyle, StrokeStyle};
use winit::window::Window;

use crate::defaults::{shadow_default, stroke_default};
use crate::fonts::get_builtin_fonts;
use crate::State;

pub mod color_picker;

pub fn render_control_panel_ui(state: &mut State, window: &Window) -> FullOutput {
    let raw_input = state.egui_state.take_egui_input(window);
    state.egui_context.run(raw_input, |ctx| {
        // Bottom panel for text input and configuration
        egui::TopBottomPanel::bottom("text_input_panel")
            .resizable(false)
            .default_height(350.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        // Text input section
                        // ui.heading("Playground");

                        ui.add_space(10.);

                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.heading("Text");
                                if egui::TextEdit::multiline(&mut state.input_text)
                                    .desired_width(400.)
                                    .desired_rows(16)
                                    .font(egui::TextStyle::Name("custom_font".into()))
                                    .show(ui)
                                    .response
                                    .changed()
                                {
                                    state.config_changed = true;
                                }
                            });
                            // ui.add_space(10.0);
                            ui.separator();
                            // Layout configuration
                            ui.vertical(|ui| {
                                ui.set_width(300.);
                                ui.heading("🎨 Basic");
                                ui.horizontal(|ui| {
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
                                        state.config_changed = true;
                                    }
                                });

                                ui.horizontal(|ui| {
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
                                                    .clicked()
                                                {
                                                    let _ = state.huozi.take();
                                                    state.config_changed = true;
                                                }
                                            }
                                        });
                                });

                                ui.add_space(10.);

                                ui.heading("⚙ Layout Configuration");
                                if ui
                                    .horizontal(|ui| {
                                        ui.disable();
                                        ui.label("Direction:")
                                            .on_disabled_hover_text("Not supported by now");
                                        ui.radio_value(
                                            &mut state.layout_config.direction,
                                            LayoutDirection::Horizontal,
                                            "Horizontal",
                                        ) | ui.radio_value(
                                            &mut state.layout_config.direction,
                                            LayoutDirection::Vertical,
                                            "Vertical",
                                        )
                                    })
                                    .inner
                                    .changed()
                                {
                                    state.config_changed = true;
                                }

                                if ui
                                    .horizontal(|ui| {
                                        ui.label("Box Width:");
                                        ui.add(egui::Slider::new(
                                            &mut state.layout_config.box_width,
                                            0.0..=1280.0,
                                        ))
                                    })
                                    .inner
                                    .changed()
                                {
                                    state.config_changed = true;
                                }

                                if ui
                                    .horizontal(|ui| {
                                        ui.label("Box Height:");
                                        ui.add(egui::Slider::new(
                                            &mut state.layout_config.box_height,
                                            0.0..=360.0,
                                        ))
                                    })
                                    .inner
                                    .changed()
                                {
                                    state.config_changed = true;
                                }

                                if ui
                                    .horizontal(|ui| {
                                        ui.label("Glyph Grid Size:");
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut state.layout_config.glyph_grid_size,
                                            )
                                            .speed(1.0)
                                            .range(8.0..=128.0),
                                        )
                                    })
                                    .inner
                                    .changed()
                                {
                                    state.config_changed = true;
                                }
                            });

                            ui.separator();

                            // Text style configuration
                            ui.vertical(|ui| {
                                ui.heading("🎨 Text Style Configuration");
                                ui.horizontal(|ui| {
                                    ui.label("Font Size:");
                                    ui.add(
                                        egui::DragValue::new(&mut state.text_config.font_size)
                                            .speed(1.0)
                                            .range(8.0..=128.0),
                                    );
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Line Height:");
                                    ui.add(
                                        egui::DragValue::new(&mut state.text_config.line_height)
                                            .speed(0.1)
                                            .range(0.5..=3.0),
                                    );
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Indent:");
                                    ui.add(
                                        egui::DragValue::new(&mut state.text_config.indent)
                                            .speed(1.0)
                                            .range(0.0..=200.0),
                                    );
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Fill Color:");
                                    if color_picker::color_picker_srgba(
                                        ui,
                                        &mut state.text_config.fill_color,
                                    )
                                    .changed()
                                    {
                                        state.config_changed = true;
                                    }
                                });

                                ui.add_space(5.0);

                                // Stroke configuration
                                ui.checkbox(&mut state.stroke_enabled, "Enable Stroke");
                                if state.stroke_enabled {
                                    ui.indent("stroke", |ui| {
                                        if state.text_config.stroke.is_none() {
                                            state.text_config.stroke = Some(stroke_default());
                                        }

                                        if let Some(stroke) = &mut state.text_config.stroke {
                                            ui.horizontal(|ui| {
                                                ui.label("Stroke Width:");
                                                ui.add(
                                                    egui::DragValue::new(&mut stroke.stroke_width)
                                                        .speed(0.1)
                                                        .range(0.0..=30.0),
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Stroke Color:");
                                                if color_picker::color_picker_srgba(
                                                    ui,
                                                    &mut stroke.stroke_color,
                                                )
                                                .changed()
                                                {
                                                    state.config_changed = true;
                                                }
                                            });
                                        }
                                    });
                                } else {
                                    state.text_config.stroke = None;
                                }

                                ui.add_space(5.0);

                                // Shadow configuration
                                ui.checkbox(&mut state.shadow_enabled, "Enable Shadow");
                                if state.shadow_enabled {
                                    ui.indent("shadow", |ui| {
                                        if state.text_config.shadow.is_none() {
                                            state.text_config.shadow = Some(shadow_default());
                                        }

                                        if let Some(shadow) = &mut state.text_config.shadow {
                                            ui.horizontal(|ui| {
                                                ui.label("Shadow Offset X:");
                                                ui.add(
                                                    egui::DragValue::new(
                                                        &mut shadow.shadow_offset_x,
                                                    )
                                                    .speed(0.5)
                                                    .range(-50.0..=50.0),
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Shadow Offset Y:");
                                                ui.add(
                                                    egui::DragValue::new(
                                                        &mut shadow.shadow_offset_y,
                                                    )
                                                    .speed(0.5)
                                                    .range(-50.0..=50.0),
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Shadow Blur:");
                                                ui.add(
                                                    egui::DragValue::new(&mut shadow.shadow_blur)
                                                        .speed(0.5)
                                                        .range(0.0..=50.0),
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Shadow Width:");
                                                ui.add(
                                                    egui::DragValue::new(&mut shadow.shadow_width)
                                                        .speed(0.1)
                                                        .range(0.0..=20.0),
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Shadow Color:");
                                                if color_picker::color_picker_srgba(
                                                    ui,
                                                    &mut shadow.shadow_color,
                                                )
                                                .changed()
                                                {
                                                    state.config_changed = true;
                                                }
                                            });
                                        }
                                    });
                                } else {
                                    state.text_config.shadow = None;
                                }
                            });
                        });
                    });
            });
    })
}
