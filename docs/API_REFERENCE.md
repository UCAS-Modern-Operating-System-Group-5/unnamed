# Search Server API 功能文档

## 概述

本文档描述了 Unnamed Search Server 的所有 RPC 接口和可用参数。服务器基于 tarpc 框架，使用 Unix Domain Socket 进行本地进程间通信。

**API 版本**: 支持两套 API
- **新 API**: 异步搜索 + Offset-based 分页（推荐，支持流式/无限滚动）
- **旧 API**: 同步搜索 + Page-based 分页（兼容）

---

## RPC 接口

### 1. `ping() -> String`

**功能**: 健康检查  
**返回**: `"Pong"`

---

## 新 API（推荐）

### 2. `start_search_async(SearchRequest) -> StartSearchResult`

**功能**: 启动异步搜索（立即返回，后台执行）  
**返回**: 
- `StartSearchResult::Started { session_id }` - 搜索已启动
- `StartSearchResult::Failed(String)` - 参数验证失败

**特点**: 
- ⚡ 不阻塞，立即返回 session_id
- 后台异步执行搜索
- 可随时通过 `fetch_results` 获取已有结果

---

### 3. `fetch_results(session_id, offset, limit) -> Option<FetchResults>`

**功能**: 获取搜索结果（offset-based，支持无限滚动）  
**参数**:
- `session_id: usize` - 会话 ID
- `offset: usize` - 从第几个结果开始（0-indexed）
- `limit: usize` - 最多返回多少个

**返回**: 
```rust
FetchResults {
    session_id: usize,
    offset: usize,           // 本次返回的起始偏移
    hits: Vec<SearchHit>,    // 结果列表
    status: SearchStatus,    // 当前状态
    has_more: bool,          // 是否还有更多 ⭐
}
```

**SearchStatus 状态**:
```rust
enum SearchStatus {
    InProgress { found_so_far: usize },  // 搜索进行中
    Completed { total_count: usize },    // 搜索完成
    Failed(String),                       // 搜索失败
    Cancelled,                            // 已取消
}
```

**`has_more` 判断逻辑**:
- `InProgress` → `true`（还在搜，肯定有更多）
- `Completed` → `offset + hits.len() < total_count`
- `Failed` / `Cancelled` → `false`

---

### 4. `cancel_search(session_id) -> bool`

**功能**: 取消搜索并释放资源  
**返回**: 
- `true` - 成功取消
- `false` - 会话不存在

---

## 旧 API（兼容）

### 5. `start_search(SearchRequest) -> SearchResult`

**功能**: 启动同步搜索（阻塞等待全部完成）  
**返回**: 
- `SearchResult::Started { session_id, total_count }` - 搜索完成
- `SearchResult::Failed(String)` - 搜索失败

**注意**: 此 API 会阻塞直到搜索完全完成

---

### 6. `get_results_page(session_id, page, page_size) -> Option<PagedResults>`

**功能**: 获取分页结果（page-based）  
**参数**:
- `session_id: usize` - 会话 ID
- `page: usize` - 页码（从 0 开始）
- `page_size: usize` - 每页大小

**返回**: 
```rust
PagedResults {
    session_id: usize,
    page: usize,
    page_size: usize,
    total_count: usize,   // 总结果数
    total_pages: usize,   // 总页数
    hits: Vec<SearchHit>,
}
```

---

## API 对比

| 特性 | 新 API (Offset) | 旧 API (Page) |
|-----|----------------|---------------|
| 首次响应 | ⚡ 立即返回 | ⏳ 等待搜索完成 |
| 分页方式 | `offset + limit` | `page + page_size` |
| 总数信息 | 可选（`has_more` 替代） | 必须等全部完成 |
| 无限滚动 | ✅ 完美支持 | ❌ 不友好 |
| 进度展示 | ✅ `InProgress { found_so_far }` | ❌ 无 |
| 适用场景 | GUI 无限滚动、大结果集 | 传统分页、小结果集 |

### 核心参数

| 参数 | 类型 | 必填 | 实现状态 | 说明 |
|-----|------|------|---------|------|
| `root_directories` | `Vec<PathBuf>` | ✅ | ✅ 完整支持 | 搜索根目录列表，必须至少提供一个 |

### 查询参数

| 参数 | 类型 | 必填 | 实现状态 | 说明 |
|-----|------|------|---------|------|
| `keywords` | `Vec<String>` | ❌ | ✅ 完整支持 | 关键词列表，使用 Tantivy 全文搜索 + jieba 中文分词 |
| `semantic_queries` | `Vec<String>` | ❌ | 🟡 部分支持 | 自然语言查询，通过 BERT 模型提取关键词后搜索 |
| `regular_expressions` | `Vec<String>` | ❌ | ❌ 未实现 | 正则表达式列表（已定义但未实现） |

**查询说明**:
- `keywords`: 直接作为 Tantivy 查询字符串，支持中文分词
- `semantic_queries`: 调用 `engine.refine_query()` 使用 BERT 提取关键词
- 至少需要提供 `keywords` 或 `semantic_queries` 之一

### 过滤参数

| 参数 | 类型 | 必填 | 实现状态 | 说明 |
|-----|------|------|---------|------|
| `include_globs` | `Vec<String>` | ❌ | ✅ 完整支持 | 文件名 glob 模式白名单，例如 `["*.txt", "*.rs"]` |
| `exclude_globs` | `Vec<String>` | ❌ | ✅ 完整支持 | 文件名 glob 模式黑名单，例如 `["target/*", ".git/*"]` |
| `semantic_threshold` | `Option<f32>` | ❌ | ✅ 完整支持 | 语义搜索最低相似度（0.0-1.0），过滤低分结果 |
| `time_accessed_range` | `Option<(SystemTime, SystemTime)>` | ❌ | ✅ 已实现 | 文件访问时间范围（通过 Query DSL `atime:` 语法） |
| `time_created_range` | `Option<(SystemTime, SystemTime)>` | ❌ | ✅ 已实现 | 文件创建时间范围（通过 Query DSL `ctime:` 语法） |
| `time_modified_range` | `Option<(SystemTime, SystemTime)>` | ❌ | ✅ 已实现 | 文件修改时间范围（通过 Query DSL `mtime:` 语法） |
| `size_range_bytes` | `Option<(u64, u64)>` | ❌ | ✅ 已实现 | 文件大小范围（通过 Query DSL `size:` 语法） |

**Glob 模式示例**:
```rust
include_globs: vec!["*.rs".to_string(), "*.toml".to_string()],  // 只搜索 Rust 和 TOML 文件
exclude_globs: vec!["target/**".to_string(), ".*/**".to_string()],  // 排除 target 和隐藏目录
```

### 展示与控制参数

| 参数 | 类型 | 必填 | 实现状态 | 说明 |
|-----|------|------|---------|------|
| `max_results` | `Option<usize>` | ❌ | ✅ 完整支持 | 最大结果数，默认无限制 |
| `sort` | `SortMode` | ✅ | 🟡 部分支持 | 排序模式（见下表） |

#### SortMode 排序模式

| 模式 | 实现状态 | 说明 |
|-----|---------|------|
| `Alphabetical` | ❌ | 按文件名字母顺序 |
| `ReverseAlphabetical` | ❌ | 按文件名字母倒序 |
| `AccessedTime` | ❌ | 按访问时间排序 |
| `CreatedTime` | ❌ | 按创建时间排序 |
| `ModifiedTime` | ❌ | 按修改时间排序 |
| `Extension` | ❌ | 按文件扩展名排序 |
| `Relevance` | ✅ | 按 AI 相关性评分排序（默认行为） |

**注**: 目前只支持 `Relevance` 排序（按搜索引擎评分）

---

## SearchResult 返回结构

### Started 成功响应

```rust
SearchResult::Started {
    session_id: 1,      // 会话 ID，用于后续分页查询
    total_count: 42,    // 总结果数
}
```

### Failed 失败响应

```rust
SearchResult::Failed("root_directories 不能为空".to_string())
```

---

## PagedResults 分页结构

```rust
PagedResults {
    session_id: 1,           // 会话 ID
    page: 1,                 // 当前页码（从 1 开始）
    page_size: 10,           // 每页大小
    total_count: 42,         // 总结果数
    total_pages: 5,          // 总页数
    hits: Vec<SearchHit>,    // 当前页的搜索结果
}
```

---

## SearchHit 结果项结构

```rust
SearchHit {
    file_path: PathBuf,             // 文件路径
    score: 0.85,                    // 相关性评分（0.0-1.0）
    snippet: "...匹配内容...",      // 摘要片段
    file_size: 1024,                // 文件大小（字节）
    modified_time: SystemTime,      // 修改时间
}
```

---

## 完整使用示例

### 🚀 推荐：无限滚动搜索（新 API）

```rust
use rpc::search::{SearchRequest, SortMode, SearchStatus, StartSearchResult};
use std::time::Duration;

// 1. 启动异步搜索
let req = SearchRequest {
    root_directories: vec![PathBuf::from("/path/to/search")],
    keywords: vec!["Rust".to_string()],
    // ... 其他参数
    sort: SortMode::Relevance,
    max_results: None,  // 不限制结果数
};

let result = client.start_search_async(context::current(), req).await?;

if let StartSearchResult::Started { session_id } = result {
    let mut offset = 0;
    let limit = 20;  // 每次获取 20 个
    
    loop {
        // 2. 获取结果（不阻塞，返回当前可用结果）
        let fetch = client.fetch_results(
            context::current(), 
            session_id, 
            offset, 
            limit
        ).await?;
        
        if let Some(result) = fetch {
            // 3. 显示结果
            for hit in &result.hits {
                display_hit(hit);
            }
            
            // 4. 检查是否还有更多
            if !result.has_more {
                if let SearchStatus::Completed { total_count } = result.status {
                    println!("搜索完成，共 {} 个结果", total_count);
                }
                break;
            }
            
            // 5. 更新 offset 继续获取
            offset += result.hits.len();
            
            // 6. 如果还在搜索中但暂时没有新结果，等待一下
            if result.hits.is_empty() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        } else {
            break;  // 会话不存在
        }
    }
    
    // 7. 清理会话
    client.cancel_search(context::current(), session_id).await?;
}
```

### 传统分页搜索（旧 API）

```rust
use rpc::search::{SearchRequest, SortMode, SearchResult};

let req = SearchRequest {
    root_directories: vec![PathBuf::from("/path/to/search")],
    keywords: vec!["Rust".to_string()],
    // ... 其他参数
    sort: SortMode::Relevance,
    max_results: Some(100),
};

// 同步搜索（阻塞等待完成）
let result = client.start_search(context::current(), req).await?;

if let SearchResult::Started { session_id, total_count } = result {
    println!("找到 {} 个结果", total_count);
    
    // 分页获取
    let page_size = 10;
    for page in 0..((total_count + page_size - 1) / page_size) {
        if let Some(results) = client.get_results_page(
            context::current(),
            session_id,
            page,
            page_size
        ).await? {
            println!("第 {} 页:", page + 1);
            for hit in results.hits {
                println!("  {:?} (score: {:.2})", hit.file_path, hit.score);
            }
        }
    }
    
    client.cancel_search(context::current(), session_id).await?;
}
            println!("{} (评分: {:.2})", hit.file_path.display(), hit.score);
            println!("  {}", hit.snippet);
        }
    }
}
```

### 高级过滤搜索

```rust
let req = SearchRequest {
    root_directories: vec![
        PathBuf::from("/home/user/projects"),
        PathBuf::from("/home/user/docs"),
    ],
    
    // 组合关键词和语义查询
    keywords: vec!["TODO".to_string()],
    semantic_queries: vec!["Find urgent tasks from last week".to_string()],
    regular_expressions: vec![],
    
    // 只搜索特定文件类型
    include_globs: vec!["*.rs".to_string(), "*.md".to_string()],
    exclude_globs: vec!["target/**".to_string(), "node_modules/**".to_string()],
    
    // 过滤低相关性结果
    semantic_threshold: Some(0.7),
    
    time_accessed_range: None,
    time_created_range: None,
    time_modified_range: None,
    size_range_bytes: None,
    
    sort: SortMode::Relevance,
    max_results: Some(50),
};
```

---

## 会话管理

### 会话生命周期

- **创建**: 调用 `start_search` 自动创建会话
- **有效期**: 30 分钟（1800 秒）
- **清理**: 
  - 自动清理：过期会话定期清除
  - 手动清理：调用 `cancel_search` 立即释放

### 并发限制

- 服务器可同时维护多个会话
- 每个会话独立管理结果和状态
- 建议客户端用完后主动调用 `cancel_search` 释放资源

---

## 配置文件

服务器配置文件位置: `~/.config/unnamed/server.toml`

```toml
# 监视目录列表（用于索引）
watch-paths = [
    "/Users/username/Documents",
    "/Users/username/Projects"
]
```

**说明**:
- `index` 命令使用 `watch-paths` 作为默认索引路径
- `search` 的 `root_directories` 参数独立于此配置
- 首次运行自动创建配置文件模板

---

## 最佳实践

### ✅ 推荐用法

1. **关键词 + Glob 过滤**
   ```rust
   keywords: vec!["function".to_string()],
   include_globs: vec!["*.rs".to_string()],
   ```

2. **语义查询 + 阈值过滤**
   ```rust
   semantic_queries: vec!["How to parse JSON".to_string()],
   semantic_threshold: Some(0.6),
   ```

3. **限制结果数**
   ```rust
   max_results: Some(100),  // 避免内存占用过大
   ```

### ⚠️ 注意事项

1. **正则表达式**: 目前未实现，使用 `keywords` + glob 替代
2. **时间/大小过滤**: 未实现，需要在客户端过滤结果
3. **排序模式**: 除 `Relevance` 外暂不支持其他模式
4. **语义查询**: 需要先索引文档，否则只返回关键词匹配结果

---

## 技术栈

- **搜索引擎**: Tantivy 0.25
- **中文分词**: jieba-rs
- **AI 模型**: Candle 0.8.2 + BAAI/bge-small-zh-v1.5 (BERT)
- **缓存**: sled 0.34 (嵌入式 KV 数据库)
- **RPC 框架**: tarpc 0.37 (Unix Domain Socket)

---

## 命令行工具

### 索引命令

```bash
# 使用配置文件中的 watch-paths
cargo run -p server -- index

# 指定路径
cargo run -p server -- index /path/to/directory
```

### 启动服务器

```bash
cargo run -p server -- serve
```

### 测试客户端

```bash
cargo run -p server --example test_client
```

---

## 错误码

| 错误信息 | 原因 | 解决方法 |
|---------|------|---------|
| `root_directories 不能为空` | 未提供搜索路径 | 添加至少一个目录到 `root_directories` |
| `没有有效的搜索条件` | 所有查询参数为空 | 提供 `keywords` 或 `semantic_queries` |
| `Failed to connect` | 服务器未启动 | 先运行 `cargo run -p server -- serve` |
| `Session not found` | 会话过期或不存在 | 检查 session_id 或重新搜索 |

---

**文档版本**: 1.0  
**最后更新**: 2026-01-10
