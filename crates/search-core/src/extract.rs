// search-core/src/extract.rs
//! 文本提取模块

use std::fs;
use std::path::Path;
use std::time::Duration;
use anyhow::{Result, Context};
use encoding_rs::{Encoding, UTF_8, GBK, GB18030};
use chardetng::EncodingDetector;

use crate::models::FileDoc;
use crate::config::CONFIG;

/// 智能读取文本文件（自动检测编码）
fn read_text_with_encoding_detection(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    
    // 先尝试UTF-8
    if let Ok(text) = std::str::from_utf8(&bytes) {
        tracing::debug!("文件使用 UTF-8 编码: {:?}", path);
        return Ok(text.to_string());
    }
    
    // 使用 chardetng 检测编码
    let mut detector = EncodingDetector::new();
    detector.feed(&bytes, true);
    let detected_encoding = detector.guess(None, true);
    
    tracing::debug!("检测到文件编码 {:?}: {:?}", detected_encoding.name(), path);
    
    // 尝试使用检测到的编码
    let (decoded, encoding_used, had_errors) = detected_encoding.decode(&bytes);
    
    if had_errors {
        tracing::warn!("文件 {:?} 使用 {} 解码时有部分错误，可能影响搜索准确性", path, encoding_used.name());
    }
    
    Ok(decoded.into_owned())
}

/// 从文件提取文本内容
pub fn extract_text(path: &Path) -> Result<FileDoc> {
    // 简单的防抖动：如果是刚创建的文件，可能还在写入中
    std::thread::sleep(Duration::from_millis(100));

    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    tracing::debug!("正在解析文件: {:?}", path);

    let content = match extension {
        "txt" | "md" | "rs" | "toml" | "json" | "yaml" | "yml" => {
            read_text_with_encoding_detection(path)?
        }
        "pdf" => {
            pdf_extract::extract_text(path).with_context(|| "无法解析 PDF")?
        }
        _ => return Err(anyhow::anyhow!("跳过不支持的文件格式: {}", extension)),
    };

    // 规范化路径
    let canonical_path = path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string();

    Ok(FileDoc {
        title: path.file_stem().unwrap().to_string_lossy().to_string(),
        content,
        path: canonical_path,
    })
}

/// 格式化内容预览
pub fn format_content_preview(content: &str) -> String {
    let preview_max_length = CONFIG.display.preview_max_length;
    let sentence_search_start = CONFIG.display.sentence_search_start;
    
    let cleaned_content = content.trim();
    if cleaned_content.is_empty() {
        return "[无文本内容]".to_string();
    }

    if cleaned_content.len() > preview_max_length {
        let sentence_endings = ['。', '！', '？', '.', '!', '?', '\n', '；', ';'];
        let mut end_pos = preview_max_length;
        let mut found_sentence_end = false;

        for i in (sentence_search_start..=preview_max_length).rev() {
            if i < cleaned_content.len() {
                if let Some(ch) = cleaned_content.chars().nth(i) {
                    if sentence_endings.contains(&ch) {
                        end_pos = i + 1;
                        found_sentence_end = true;
                        break;
                    }
                }
            }
        }

        if !found_sentence_end {
            end_pos = preview_max_length;
            for i in ((preview_max_length - sentence_search_start)..=preview_max_length).rev() {
                if i < cleaned_content.len() {
                    if let Some(ch) = cleaned_content.chars().nth(i) {
                        if ch.is_whitespace() || ch == '，' || ch == '。' || ch == '；' {
                            end_pos = i;
                            break;
                        }
                    }
                }
            }
        }

        while end_pos > 0 && !cleaned_content.is_char_boundary(end_pos) {
            end_pos -= 1;
        }

        if end_pos == 0 { end_pos = preview_max_length; }
        format!("{}...", &cleaned_content[..end_pos])
    } else {
        cleaned_content.to_string()
    }
}

/// 文本提取器
pub struct TextExtractor;

impl TextExtractor {
    pub fn new() -> Self {
        Self
    }
    
    /// 提取文件文本内容
    pub fn extract(&self, path: &Path) -> Result<String> {
        let file_doc = extract_text(path)?;
        Ok(file_doc.content)
    }
    
    /// 提取并返回完整的 FileDoc
    pub fn extract_doc(&self, path: &Path) -> Result<FileDoc> {
        extract_text(path)
    }
    
    /// 检查是否支持该文件类型
    pub fn is_supported(&self, path: &Path) -> bool {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        matches!(extension, "txt" | "md" | "rs" | "pdf" | "toml" | "json" | "yaml" | "yml")
    }
}

impl Default for TextExtractor {
    fn default() -> Self {
        Self::new()
    }
}
