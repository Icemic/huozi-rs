//! 字体来源列表控件。

use huozi::FontSourceKind;

use crate::FontFallback;
use crate::State;

/// 绘制字体来源列表：每项可启停、切换角色、上下移动与移除，并可追加未在列表中的字体。
///
/// 列表决定 `Huozi` 的字体目录与图集，因此列表变化时丢弃当前实例，让下一次布局按新列表重建。
pub fn render_fonts_ui(state: &mut State, ui: &mut egui::Ui) {
    ui.label("Fonts:");

    let mut changed = false;
    ui.allocate_ui_with_layout(
        egui::vec2(300.0, 260.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    changed |= render_font_list(state, ui);
                    ui.separator();
                    changed |= render_add_font_row(state, ui);
                });
        },
    );

    if changed {
        state.huozi = None;
        state.config_changed = true;
    }
}

/// 绘制可滚动的字体来源列表；返回列表是否发生变化。
fn render_font_list(state: &mut State, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    let mut move_font = None;
    let mut remove_font = None;

    egui::ScrollArea::vertical()
        .max_height(100.0)
        .show(ui, |ui| {
            let last_index = state.font_fallbacks.len().saturating_sub(1);
            for (index, font) in state.font_fallbacks.iter_mut().enumerate() {
                let display_name = state
                    .font_files
                    .iter()
                    .find(|file| file.name == font.name)
                    .map(|file| file.display_name.as_str())
                    .unwrap_or(&font.name);
                ui.horizontal(|ui| {
                    ui.add_sized([200.0, 18.0], egui::Label::new(display_name).truncate())
                        .on_hover_text(display_name);

                    if ui
                        .add_sized(
                            [32.0, 18.0],
                            egui::Button::new(if font.enabled { "On" } else { "Off" }),
                        )
                        .on_hover_text("Enable or disable this font")
                        .clicked()
                    {
                        font.enabled = !font.enabled;
                        changed = true;
                    }

                    if ui
                        .add(
                            egui::Button::new(kind_label(font.kind))
                                .min_size(egui::vec2(18.0, 18.0)),
                        )
                        .on_hover_text("Cycle kind: unset, CJK, Western")
                        .clicked()
                    {
                        font.kind = next_kind(font.kind);
                        changed = true;
                    }

                    if ui
                        .add_enabled(
                            index > 0,
                            egui::Button::new("↑").min_size(egui::vec2(18.0, 18.0)),
                        )
                        .clicked()
                    {
                        move_font = Some((index, index - 1));
                    }
                    if ui
                        .add_enabled(
                            index < last_index,
                            egui::Button::new("↓").min_size(egui::vec2(18.0, 18.0)),
                        )
                        .clicked()
                    {
                        move_font = Some((index, index + 1));
                    }
                    if ui
                        .add(egui::Button::new("×").min_size(egui::vec2(18.0, 18.0)))
                        .on_hover_text("Remove")
                        .clicked()
                    {
                        remove_font = Some(index);
                    }
                });
            }
        });

    // 移动与移除在遍历结束后统一应用，避免在迭代中修改列表。
    if let Some((from, to)) = move_font {
        state.font_fallbacks.swap(from, to);
        changed = true;
    }
    if let Some(index) = remove_font {
        state.font_fallbacks.remove(index);
        changed = true;
    }
    changed
}

/// 绘制「选择并追加字体」一行；返回列表是否发生变化。
fn render_add_font_row(state: &mut State, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
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
                    if !state
                        .font_fallbacks
                        .iter()
                        .any(|fallback| fallback.name == font.name)
                    {
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
                egui::Button::new("+").min_size(egui::vec2(18.0, 18.0)),
            )
            .clicked()
        {
            state.font_fallbacks.push(FontFallback {
                name: state
                    .font_to_add
                    .take()
                    .expect("add button requires a font"),
                kind: None,
                enabled: true,
            });
            changed = true;
        }
    });
    changed
}

/// 角色标记：未声明、CJK 或 Western。
fn kind_label(kind: Option<FontSourceKind>) -> &'static str {
    match kind {
        None => "*",
        Some(FontSourceKind::Cjk) => "C",
        Some(FontSourceKind::Western) => "W",
    }
}

/// 角色按钮的循环顺序：未声明 → CJK → Western → 未声明。
fn next_kind(kind: Option<FontSourceKind>) -> Option<FontSourceKind> {
    match kind {
        None => Some(FontSourceKind::Cjk),
        Some(FontSourceKind::Cjk) => Some(FontSourceKind::Western),
        Some(FontSourceKind::Western) => None,
    }
}
