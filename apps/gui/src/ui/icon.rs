macro_rules! icon_image {
    // $ui: The UI object
    // $name: The string literal for the filename (e.g., "sparkles.svg")
    // $size: The float size
    ($name:literal, $size:expr) => {
        egui::Image::new(egui::include_image!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/icons/",
            $name
        ))).fit_to_exact_size(egui::vec2($size, $size))
    };
}
pub(crate) use icon_image;
