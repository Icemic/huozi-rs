use egui::{Grid, Ui};

pub fn render_grid_ui<R>(
    id_salt: impl std::hash::Hash,
    ui: &mut Ui,
    add_contents: impl FnOnce(&mut Ui) -> R,
) {
    ui.horizontal_top(|ui| {
        Grid::new(id_salt)
            .min_col_width(200.)
            .num_columns(2)
            .spacing([0.0, 6.0])
            .striped(false)
            .show(ui, add_contents)
    });
}
