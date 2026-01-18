// search-core/src/schema/mod.rs
//! Schema 模块 - Tantivy 索引结构定义
//! 
//! 统一管理索引字段定义，避免魔法字符串分散在代码各处

pub mod fields;
pub mod document;
pub mod builder;

pub use fields::*;
pub use document::IndexDocument;
pub use builder::{build_schema, SchemaFields};
