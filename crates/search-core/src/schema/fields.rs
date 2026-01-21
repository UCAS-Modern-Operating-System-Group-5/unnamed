// search-core/src/schema/fields.rs
//! 字段名常量定义
//! 
//! 统一管理所有 Schema 字段名，避免魔法字符串

// ============== 已启用字段 ==============

/// 文件标题（通常是文件名，不含扩展名）
pub const FIELD_TITLE: &str = "title";

/// 文件内容（全文检索主字段）
pub const FIELD_BODY: &str = "body";

/// 完整文件路径（唯一标识符）
pub const FIELD_PATH: &str = "path";

/// AI 生成的标签（空格分隔）
pub const FIELD_TAGS: &str = "tags";

/// 文件大小（字节）
pub const FIELD_FILE_SIZE: &str = "file_size";

/// 文件修改时间（Unix 时间戳秒）
pub const FIELD_MODIFIED_TIME: &str = "modified_time";

// ============== 待启用字段 ==============
// 以下字段已在 RPC SearchRequest 中定义过滤条件，但 Schema 尚未支持
// 启用后需要同步修改: builder.rs, document.rs, indexer.rs, rpc_compat.rs

/// 父目录路径（用于目录过滤）
/// 
/// **状态**: 🔴 未启用
/// **用途**: 支持 "只搜索某目录下的文件" 功能
/// **RPC 对应**: SearchRequest.root_directories（当前在内存中过滤）
#[allow(dead_code)]
pub const FIELD_PARENT_PATH: &str = "parent_path";

/// 文件名（不含路径，含扩展名）
/// 
/// **状态**: 🔴 未启用
/// **用途**: 支持按文件名精确搜索
#[allow(dead_code)]
pub const FIELD_FILENAME: &str = "filename";

/// 文件类型/扩展名（不含点号，如 "rs", "md"）
/// 
/// **状态**: 🔴 未启用
/// **用途**: 支持按文件类型过滤
/// **RPC 对应**: SearchRequest.include_globs（当前用 glob 匹配）
#[allow(dead_code)]
pub const FIELD_FILE_TYPE: &str = "file_type";

/// 文件创建时间（Unix 时间戳秒）
/// 
/// **状态**: � 已启用
/// **用途**: 支持按创建时间范围过滤
/// **RPC 对应**: SearchRequest.time_created_range
pub const FIELD_CREATED_TIME: &str = "created_time";

/// 文件访问时间（Unix 时间戳秒）
/// 
/// **状态**: � 已启用
/// **用途**: 支持按访问时间范围过滤
/// **RPC 对应**: SearchRequest.time_accessed_range
pub const FIELD_ACCESSED_TIME: &str = "accessed_time";

/// 索引时间（文档被索引的时间）
/// 
/// **状态**: 🔴 未启用
/// **用途**: 追踪索引更新，增量同步
#[allow(dead_code)]
pub const FIELD_INDEXED_TIME: &str = "indexed_time";
