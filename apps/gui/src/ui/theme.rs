use egui::{Context, FontData, FontDefinitions, FontFamily};
use font_kit::{
    family_name::FamilyName, handle::Handle, properties::Properties, source::SystemSource,
};
use log::{debug, info};
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


// pub fn setup_fonts(ctx: &Context) {
//     let mut fonts = FontDefinitions::default();

//     // This method seems bump memory usage by 50 MB
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

/// We don't load system Chinese font since it will takes generally 50~70MB memory
/// Embedding Maple Mono NL CN font takes around 10MB memory (Only loads regular weight font)
pub fn setup_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::empty();
    let font_name = "Maple Mono NL CN".to_string();
    
    fonts.font_data.insert(font_name.clone(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../assets/MapleMonoNL-CN-Regular.ttf"
        ))),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, font_name.clone());

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, font_name.clone());
            
    ctx.set_fonts(fonts);
}
