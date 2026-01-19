macro_rules! icon_image {
    ($name:literal, $size:expr) => {{
        let img = egui::Image::new(egui::include_image!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/icons/",
            $name
        )));

        if let Some(s) = $size {
            img.fit_to_exact_size(egui::vec2(s, s))
        } else {
            img
        }
    }};
}
pub(crate) use icon_image;
