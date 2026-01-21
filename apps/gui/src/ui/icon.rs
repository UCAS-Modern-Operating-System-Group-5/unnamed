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

/// File type enumeration for icon mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Pdf,
    Doc,
    Text,
    Markdown,
    Code,
    Generic,
}

impl FileType {
    /// Determine file type from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "pdf" => FileType::Pdf,
            "doc" | "docx" => FileType::Doc,
            "txt" => FileType::Text,
            "md" | "markdown" => FileType::Markdown,
            "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp" | "java" | "go" | "rb" | "php" | "swift" | "kt" | "scala" | "sh" | "bash" | "zsh" | "toml" | "yaml" | "yml" | "json" | "xml" | "html" | "css" | "scss" | "sass" | "less" => FileType::Code,
            _ => FileType::Generic,
        }
    }

    /// Get file type from a path
    pub fn from_path(path: &std::path::Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(Self::from_extension)
            .unwrap_or(FileType::Generic)
    }

    /// Get display name for the file type
    pub fn display_name(&self) -> &'static str {
        match self {
            FileType::Pdf => "PDF",
            FileType::Doc => "Document",
            FileType::Text => "Text",
            FileType::Markdown => "Markdown",
            FileType::Code => "Code",
            FileType::Generic => "File",
        }
    }
}

/// Get the file type icon image for a given file type
pub fn file_type_icon(file_type: FileType, size: Option<f32>) -> egui::Image<'static> {
    match file_type {
        FileType::Pdf => icon_image!("file-pdf.svg", size),
        FileType::Doc => icon_image!("file-doc.svg", size),
        FileType::Text => icon_image!("file-txt.svg", size),
        FileType::Markdown => icon_image!("file-md.svg", size),
        FileType::Code => icon_image!("file-code.svg", size),
        FileType::Generic => icon_image!("file-generic.svg", size),
    }
}

/// Get file type icon from path
pub fn file_icon_from_path(path: &std::path::Path, size: Option<f32>) -> egui::Image<'static> {
    file_type_icon(FileType::from_path(path), size)
}
