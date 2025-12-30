// Currently egui doesn't support automatically load system font to cover missing
// glyph
// Issue to track: https://github.com/emilk/egui/issues/5233

use egui::{Context, FontData, FontDefinitions, FontFamily};
use font_kit::{
    family_name::FamilyName, handle::Handle, properties::Properties, source::SystemSource,
};
use tracing::{debug, info};
use std::sync::Arc;



#[allow(dead_code)]
// Reference: https://github.com/woelper/oculante/blob/66e00785f13ef008e516d790b88ec34436188d24/src/ui/theme.rs#L110-L133
/// Attempt to load a system font by any of the given `family_names`, returning the first match.
fn load_font_family(family_names: &[&str]) -> Option<Vec<u8>> {
    let system_source = SystemSource::new();
    for &name in family_names {
        let font_handle = system_source.select_best_match(
            &[FamilyName::Title(name.to_string())],
            &Properties::new(),
        );
        match font_handle {
            Ok(h) => match &h {
                Handle::Memory { bytes, .. } => {
                    info!("Loaded {name} from memory.");
                    return Some(bytes.to_vec());
                }
                Handle::Path { path, .. } => {
                    info!("Loaded {name} from path: {:?}", path);
                    if let Ok(data) = std::fs::read(path) {
                        return Some(data);
                    }
                }
            },
            Err(e) => debug!("Could not load {}: {:?}", name, e),
        }
    }
    None
}

#[allow(dead_code)]
pub fn load_system_chinese_font() -> Result<FontData, String> {
    debug!("Attempting to load sys fonts");

    let font_families = vec![
        "Noto Sans CJK SC",
        "Microsoft YaHei",
        "Noto Sans SC",
        "WenQuanYi Zen Hei",
        "PingFang SC",
        "Heiti SC",
        "Songti SC",
        "SimSun",
        "Noto Sans SC",
        "Source Han Sans CN",
    ];

    if let Some(font_data) = load_font_family(&font_families) {
        return Ok(FontData::from_owned(font_data));
    }

    Err("No Chinese font founded".to_string())
}


// This methods find NotoSansCJK-VF.otf.ttc (~33MB) font on my system and takes
// ~66MB ((154536 - 86876) / 1024) memory
// use log::warn;
// pub fn setup_fonts(ctx: &Context) {
//     let mut fonts = FontDefinitions::default();

//     match load_system_chinese_font() {
//         Ok(chinese_font_data) => {
//             fonts.font_data.insert("chinese".to_owned(),
//                 Arc::new(chinese_font_data)
//             );

//             fonts
//                 .families
//                 .entry(FontFamily::Proportional)
//                 .or_default()
//                 .insert(0, "chinese".to_owned());

//             fonts
//                 .families
//                 .entry(FontFamily::Monospace)
//                 .or_default()
//                 .insert(0, "chinese".to_owned());
            
//             ctx.set_fonts(fonts);
//         }
//         Err(e) => {
//             warn!("Couldn't load a Chinese font! Error: {:?}", e);
//         }
//     }
// }

// It takes no additional memory since font data are inside the `.rodata` segment.
// The cost is the increased executable size.
pub fn setup_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::empty();
    
    // We only load regular weight font since egui currently doesn't support
    // font weight. Related issues:
    // https://github.com/emilk/egui/issues/3218
    // https://github.com/emilk/egui/issues/3218#issuecomment-3173550321
    fonts.font_data.insert("Noto Sans".to_string(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../assets/NotoSansCJKsc-Regular.otf"
        )))
    );

    fonts.font_data.insert("Noto Sans Mono".to_string(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../assets/NotoSansMonoCJKsc-Regular.otf"
        )))
    );


    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Noto Sans".to_string());

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "Noto Sans Mono".to_string());
            
    ctx.set_fonts(fonts);
}
