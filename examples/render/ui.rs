mod color_picker;
mod grid;
mod switch;

use egui::FullOutput;
use winit::window::Window;

use crate::State;
use crate::defaults::{shadow_default, stroke_default};
use crate::ui::grid::render_grid_ui;
use crate::ui::switch::toggle;
use huozi::layout::ParagraphAlignment;

pub fn render_control_panel_ui(state: &mut State, window: &Window) -> FullOutput {
    let raw_input = state.egui_state.take_egui_input(window);
    state.egui_context.run_ui(raw_input, |ui| {
        if let Some(notice) = state.interaction_notice.clone() {
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

        // Bottom panel for text input and configuration
        egui::Panel::bottom("text_input_panel")
            .resizable(true)
            .default_size(360.0)
            .show(ui, |ui| {
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

                    let mut font_fallbacks_changed = false;
                    ui.vertical(|ui| {
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

                        ui.label("Fonts:");
                        ui.allocate_ui_with_layout(
                            egui::vec2(300.0, 260.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::Frame::group(ui.style())
                                    .inner_margin(egui::Margin::same(4))
                                    .show(ui, |ui| {
                                        let mut move_font = None;
                                        let mut remove_font = None;
                                        egui::ScrollArea::vertical().max_height(100.0).show(
                                            ui,
                                            |ui| {
                                                let last_index =
                                                    state.font_fallbacks.len().saturating_sub(1);
                                                for (index, font) in
                                                    state.font_fallbacks.iter_mut().enumerate()
                                                {
                                                    let display_name = state
                                                        .font_files
                                                        .iter()
                                                        .find(|file| file.name == font.name)
                                                        .map(|file| file.display_name.as_str())
                                                        .unwrap_or(&font.name);
                                                    ui.horizontal(|ui| {
                                                        ui.add_sized(
                                                            [200.0, 18.0],
                                                            egui::Label::new(display_name)
                                                                .truncate(),
                                                        )
                                                        .on_hover_text(display_name);
                                                        if ui
                                                            .add_sized(
                                                                [32.0, 18.0],
                                                                egui::Button::new(
                                                                    if font.enabled {
                                                                        "On"
                                                                    } else {
                                                                        "Off"
                                                                    },
                                                                ),
                                                            )
                                                            .on_hover_text(
                                                                "Enable or disable this font",
                                                            )
                                                            .clicked()
                                                        {
                                                            font.enabled = !font.enabled;
                                                            font_fallbacks_changed = true;
                                                        }
                                                        let kind_label = match font.kind {
                                                            None => "*",
                                                            Some(huozi::FontSourceKind::Cjk) => "C",
                                                            Some(huozi::FontSourceKind::Latin) => {
                                                                "L"
                                                            }
                                                        };
                                                        if ui
                                                            .add(
                                                                egui::Button::new(kind_label)
                                                                    .min_size(egui::vec2(
                                                                        18.0, 18.0,
                                                                    )),
                                                            )
                                                            .on_hover_text(
                                                                "Cycle kind: unset, CJK, Latin",
                                                            )
                                                            .clicked()
                                                        {
                                                            font.kind = match font.kind {
                                                                None => {
                                                                    Some(huozi::FontSourceKind::Cjk)
                                                                }
                                                                Some(
                                                                    huozi::FontSourceKind::Cjk,
                                                                ) => Some(
                                                                    huozi::FontSourceKind::Latin,
                                                                ),
                                                                Some(
                                                                    huozi::FontSourceKind::Latin,
                                                                ) => None,
                                                            };
                                                            font_fallbacks_changed = true;
                                                        }
                                                        if ui
                                                            .add_enabled(
                                                                index > 0,
                                                                egui::Button::new("↑").min_size(
                                                                    egui::vec2(18.0, 18.0),
                                                                ),
                                                            )
                                                            .clicked()
                                                        {
                                                            move_font = Some((index, index - 1));
                                                        }
                                                        if ui
                                                            .add_enabled(
                                                                index < last_index,
                                                                egui::Button::new("↓").min_size(
                                                                    egui::vec2(18.0, 18.0),
                                                                ),
                                                            )
                                                            .clicked()
                                                        {
                                                            move_font = Some((index, index + 1));
                                                        }
                                                        if ui
                                                            .add(
                                                                egui::Button::new("×").min_size(
                                                                    egui::vec2(18.0, 18.0),
                                                                ),
                                                            )
                                                            .on_hover_text("Remove")
                                                            .clicked()
                                                        {
                                                            remove_font = Some(index);
                                                        }
                                                    });
                                                }
                                            },
                                        );
                                        if let Some((from, to)) = move_font {
                                            state.font_fallbacks.swap(from, to);
                                            font_fallbacks_changed = true;
                                        }
                                        if let Some(index) = remove_font {
                                            state.font_fallbacks.remove(index);
                                            font_fallbacks_changed = true;
                                        }

                                        ui.separator();
                                        ui.horizontal(|ui| {
                                            let selected_text = state
                                                .font_to_add
                                                .as_deref()
                                                .and_then(|name| {
                                                    state
                                                        .font_files
                                                        .iter()
                                                        .find(|font| font.name == name)
                                                        .map(|font| font.display_name.as_str())
                                                })
                                                .unwrap_or("Select a font");
                                            egui::ComboBox::from_id_salt("add_font_fallback")
                                                .width(182.0)
                                                .selected_text(selected_text)
                                                .show_ui(ui, |ui| {
                                                    for font in &state.font_files {
                                                        if !state.font_fallbacks.iter().any(
                                                            |fallback| fallback.name == font.name,
                                                        ) {
                                                            ui.selectable_value(
                                                                &mut state.font_to_add,
                                                                Some(font.name.to_owned()),
                                                                &font.display_name,
                                                            );
                                                        }
                                                    }
                                                });
                                            if ui
                                                .add_enabled(
                                                    state.font_to_add.is_some(),
                                                    egui::Button::new("+")
                                                        .min_size(egui::vec2(18.0, 18.0)),
                                                )
                                                .clicked()
                                            {
                                                state.font_fallbacks.push(crate::FontFallback {
                                                    name: state
                                                        .font_to_add
                                                        .take()
                                                        .expect("add button requires a font"),
                                                    kind: None,
                                                    enabled: true,
                                                });
                                                font_fallbacks_changed = true;
                                            }
                                        });
                                    });
                            },
                        );

                        ui.add_space(10.);
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
                                .selected_text(match state.layout_config.align {
                                    ParagraphAlignment::Start => "Start",
                                    ParagraphAlignment::Center => "Center",
                                    ParagraphAlignment::End => "End",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut state.layout_config.align,
                                        ParagraphAlignment::Start,
                                        "Start",
                                    );
                                    ui.selectable_value(
                                        &mut state.layout_config.align,
                                        ParagraphAlignment::Center,
                                        "Center",
                                    );
                                    ui.selectable_value(
                                        &mut state.layout_config.align,
                                        ParagraphAlignment::End,
                                        "End",
                                    );
                                });
                            ui.end_row();
                        });
                    });
                    if font_fallbacks_changed {
                        state.huozi.take();
                        state.config_changed = true;
                    }

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

                        ui.label("Font Weight:");
                        ui.add(egui::Slider::new(
                            &mut state.text_config.font_weight,
                            250..=900,
                        ));
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
