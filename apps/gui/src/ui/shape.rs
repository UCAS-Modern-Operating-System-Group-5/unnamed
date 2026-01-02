use egui::{Response, Sense, TextStyle, Ui, vec2, Color32};

pub fn inline_filled_circle(ui: &mut Ui, radius: f32, color: Color32) -> Response {
    let row_height = ui.text_style_height(&TextStyle::Body);
    let (rect, response) =
        ui.allocate_exact_size(vec2(radius * 2.0, row_height), Sense::hover());
    ui.painter().circle_filled(
        rect.center(),
        radius.min(row_height),
        color,
    );
    response
}
