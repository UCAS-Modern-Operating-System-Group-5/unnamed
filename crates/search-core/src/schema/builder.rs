// search-core/src/schema/builder.rs
//! Schema 构建器
//! 
//! 构建 Tantivy 索引 Schema，统一管理字段配置

use tantivy::schema::*;
use super::fields::*;

/// 构建 Tantivy Schema
/// 
/// # 已启用字段
/// - `title`: 文件标题，中文分词，存储
/// - `body`: 文件内容，中文分词，存储
/// - `path`: 文件路径，精确匹配，存储
/// - `tags`: AI 标签，中文分词，存储
/// - `file_size`: 文件大小，快速过滤，存储
/// - `modified_time`: 修改时间，快速过滤，存储
/// 
/// # 待启用字段
/// 见 `fields.rs` 中的注释
pub fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // 中文分词配置（用于 title, body, tags）
    let text_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("jieba")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions)
        )
        .set_stored();

    // ============== 已启用字段 ==============
    
    // 文本字段（支持中文分词）
    schema_builder.add_text_field(FIELD_TITLE, text_options.clone());
    schema_builder.add_text_field(FIELD_BODY, text_options.clone());
    schema_builder.add_text_field(FIELD_TAGS, text_options.clone());
    
    // 路径字段（精确匹配，不分词）
    schema_builder.add_text_field(FIELD_PATH, STRING | STORED);
    
    // 数值字段（支持范围查询和排序）
    schema_builder.add_u64_field(FIELD_FILE_SIZE, FAST | STORED);
    schema_builder.add_u64_field(FIELD_MODIFIED_TIME, FAST | STORED);

    // ============== 待启用字段 ==============
    // 取消下方注释并在 document.rs 中添加对应字段即可启用
    
    // // 父目录路径（精确匹配，用于目录过滤）
    // schema_builder.add_text_field(FIELD_PARENT_PATH, STRING | STORED);
    // 
    // // 文件名（精确匹配）
    // schema_builder.add_text_field(FIELD_FILENAME, STRING | STORED);
    // 
    // // 文件类型（精确匹配，用于类型过滤）
    // schema_builder.add_text_field(FIELD_FILE_TYPE, STRING | STORED);
    // 
    // // 时间字段（支持范围查询）
    // schema_builder.add_u64_field(FIELD_CREATED_TIME, FAST | STORED);
    // schema_builder.add_u64_field(FIELD_ACCESSED_TIME, FAST | STORED);
    // schema_builder.add_u64_field(FIELD_INDEXED_TIME, FAST | STORED);

    schema_builder.build()
}

/// Schema 字段辅助结构
/// 
/// 缓存字段引用，避免重复查找
pub struct SchemaFields {
    pub title: Field,
    pub body: Field,
    pub path: Field,
    pub tags: Field,
    pub file_size: Field,
    pub modified_time: Field,
    
    // 待启用
    // pub parent_path: Field,
    // pub filename: Field,
    // pub file_type: Field,
    // pub created_time: Field,
    // pub accessed_time: Field,
    // pub indexed_time: Field,
}

impl SchemaFields {
    /// 从 Schema 中提取所有字段引用
    pub fn from_schema(schema: &Schema) -> Self {
        Self {
            title: schema.get_field(FIELD_TITLE).expect("missing title field"),
            body: schema.get_field(FIELD_BODY).expect("missing body field"),
            path: schema.get_field(FIELD_PATH).expect("missing path field"),
            tags: schema.get_field(FIELD_TAGS).expect("missing tags field"),
            file_size: schema.get_field(FIELD_FILE_SIZE).expect("missing file_size field"),
            modified_time: schema.get_field(FIELD_MODIFIED_TIME).expect("missing modified_time field"),
            // parent_path: schema.get_field(FIELD_PARENT_PATH).expect("missing parent_path field"),
            // filename: schema.get_field(FIELD_FILENAME).expect("missing filename field"),
            // file_type: schema.get_field(FIELD_FILE_TYPE).expect("missing file_type field"),
            // created_time: schema.get_field(FIELD_CREATED_TIME).expect("missing created_time field"),
            // accessed_time: schema.get_field(FIELD_ACCESSED_TIME).expect("missing accessed_time field"),
            // indexed_time: schema.get_field(FIELD_INDEXED_TIME).expect("missing indexed_time field"),
        }
    }
}
