// Reference: Egui's EasyMask demo and code_editor demo
use crate::constants;
use egui::{
    Color32,
    text::{LayoutJob, TextFormat},
};
use query::lexer::{Token, prelude::*};

/// A simple search query highligher which memoizing previous output to save CPU
/// In practice, a search query is short and and it should be fast enough not to
/// need any caching.
#[derive(Default)]
pub struct MemoizedQueryHighligher {
    style: egui::Style,
    code: String,
    output: LayoutJob,
}

impl MemoizedQueryHighligher {
    pub fn highlight(&mut self, egui_style: &egui::Style, code: &str) -> LayoutJob {
        if (&self.style, self.code.as_str()) != (egui_style, code) {
            self.style = egui_style.clone();
            code.clone_into(&mut self.code);
            self.output = highlight_query(egui_style, code);
        }
        self.output.clone()
    }
}

fn highlight_query(egui_style: &egui::Style, text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    
    let whitespace_color = Color32::TRANSPARENT;
    let field_color = egui_style.visuals.hyperlink_color;
    let text_color = egui_style.visuals.widgets.inactive.fg_stroke.color;
    let delimeter_color = egui_style.visuals.widgets.active.bg_fill;
    let error_color = egui_style.visuals.error_fg_color;
    let font_id = egui::TextStyle::Name(constants::TEXT_STYLE_SEARCH_BAR.into()).resolve(egui_style);
    
    let tokens: Vec<_> = Token::lexer(text).spanned().collect();
    let mut last_end = 0;
    for (i, (token_result, span)) in tokens.iter().enumerate() {
        // Handle whitespace, which is ignored by our lexer
        if span.start > last_end {
            job.append(
                &text[last_end..span.start],
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color: whitespace_color,
                    ..Default::default()
                },
            );
        }

        let fg_color = match token_result {
            Ok(token) => {
                match token {
                    Token::And | Token::Or | Token::Not => delimeter_color,
                    Token::Colon => delimeter_color,
                    Token::LParen | Token::RParen => delimeter_color,
                    Token::QuotedText(_) => text_color,
                    Token::Text(_) => {
                        if matches!(tokens.get(i + 1), Some((Ok(Token::Colon), _))) {
                            // field
                            field_color
                        } else {
                            // value
                            text_color
                        }
                    },
                }
            },
            Err(_) => error_color,
        };

        job.append(
            &text[span.start..span.end],
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: fg_color,
                ..Default::default()
            }
        );

        last_end = span.end;
    }

    // Handle trailing whitespace
    if last_end < text.len() {
        job.append(
            &text[last_end..],
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: whitespace_color,
                ..Default::default()
            },
        );

    }
    
    job
}
