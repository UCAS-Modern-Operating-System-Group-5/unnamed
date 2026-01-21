# Search Server API 功能文档

## 概述

本文档描述了 Unnamed Search Server 的所有 RPC 接口和可用参数。服务器基于 tarpc 框架，使用 Unix Domain Socket 进行本地进程间通信。

**API 特点**:
- 异步搜索 + Offset-based 分页
- 支持流式返回和无限滚动
- 使用 UUID 作为会话标识

---

## RPC 接口

### 1. `ping() -> String`

**功能**: 健康检查  
**返回**: `"Pong"`

---

### 2. `start_search(SearchRequest) -> SResult<Uuid>`

**功能**: 启动异步搜索（立即返回，后台执行）  
**返回**: 
- `Ok(Uuid)` - 搜索已启动，返回会话 ID
- `Err(SearchErrorKind)` - 参数验证失败

**特点**: 
- ⚡ 不阻塞，立即返回 session_id
- 后台异步执行搜索
- 可随时通过 `fetch_search_results` 获取已有结果

---

### 3. `search_status(session_id: Uuid) -> (Uuid, SResult<SearchStatus>)`

**功能**: 查询搜索会话状态  
**返回**: 
- `(session_id, Ok(SearchStatus))` - 当前状态
- `(session_id, Err(SearchErrorKind::SessionNotExists))` - 会话不存在

**SearchStatus 状态**:
```rust
enum SearchStatus {
    InProgress { found_so_far: u64 },  // 搜索进行中
    Completed { total_count: u64 },    // 搜索完成
    Failed(SearchErrorKind),           // 搜索失败
    Cancelled,                         // 已取消
}
```

---

### 4. `fetch_search_results(FetchSearchResultsRequest) -> (Uuid, SResult<FetchResults>)`

**功能**: 获取搜索结果（offset-based，支持无限滚动）  
**参数**:
```rust
FetchSearchResultsRequest {
    session_id: Uuid,   // 会话 ID
    offset: usize,      // 从第几个结果开始（0-indexed）
    limit: usize,       // 最多返回多少个
}
```

**返回**: 
```rust
FetchResults {
    offset: u64,             // 本次返回的起始偏移
    hits: Vec<SearchHit>,    // 结果列表
    has_more: bool,          // 是否还有更多 ⭐
}
```

**`has_more` 判断逻辑**:
- `InProgress` → `true`（还在搜，肯定有更多）
- `Completed` → `offset + hits.len() < total_count`
- `Failed` / `Cancelled` → `false`

---

### 5. `cancel_search(session_id: Uuid) -> (Uuid, SResult<()>)`

**功能**: 取消搜索并释放资源  
**返回**: 
- `(session_id, Ok(()))` - 成功取消
- `(session_id, Err(SearchErrorKind::SessionNotExists))` - 会话不存在

---

## SearchRequest 请求结构

```rust
SearchRequest {
    query: String,           // 搜索查询字符串
    search_mode: SearchMode, // 搜索模式
}
```

### SearchMode 搜索模式

| 模式 | 说明 |
|-----|------|
| `Natural` | 自然语言搜索，使用 AI 语义理解 |
| `Rule` | 规则搜索，支持精确匹配、正则、路径过滤等 |

### Rule 模式查询语法（Query DSL）

Rule 模式支持丰富的查询语法：

| 语法 | 示例 | 说明 |
|-----|------|------|
| 关键词 | `rust` | 搜索包含关键词的文件 |
| AND | `rust AND tokio` | 同时包含两个词 |
| OR | `rust OR go` | 包含任一词 |
| NOT | `rust NOT async` | 包含 rust 但不含 async |
| 正则 | `/pattern/` | 使用正则表达式匹配 |
| 路径 | `root:/home/user` | 限定搜索路径 |
| Glob | `*.rs` | 文件名匹配 |
| 大小 | `size:>1MB` | 文件大小过滤 |
| 修改时间 | `mtime:<1w` | 最近一周修改 |
| 创建时间 | `ctime:>2024-01-01` | 创建时间过滤 |
| 访问时间 | `atime:<30d` | 最近 30 天访问 |

**复合查询示例**：
```
*.rs AND size:<100KB AND mtime:<1w root:/home/dev/projects
```

---

## SearchHit 结果项结构

```rust
SearchHit {
    file_path: PathBuf,             // 文件路径
    score: Option<f32>,             // 相关性评分（仅自然语言搜索）
    preview: String,                // 摘要片段
    file_size: u64,                 // 文件大小（字节）
    access_time: u64,               // 访问时间（Unix 时间戳）
    modified_time: u64,             // 修改时间（Unix 时间戳）
    create_time: u64,               // 创建时间（Unix 时间戳）
}
```

## SearchErrorKind 错误类型

```rust
enum SearchErrorKind {
    SessionNotExists,              // 会话不存在
    SessionAlreadyCancelled,       // 会话已被取消
    InvalidQuery(ValidationError), // 查询语法错误
    OperateOnAlreadyFailedSearch,  // 操作已失败的搜索
}
```

---

## 完整使用示例

### 无限滚动搜索示例

```rust
use rpc::search::{SearchRequest, SearchMode, SearchStatus, FetchSearchResultsRequest};
use std::time::Duration;

// 1. 启动异步搜索
let req = SearchRequest {
    query: "Rust programming".to_string(),
    search_mode: SearchMode::Natural,  // 或 SearchMode::Rule
};

let result = client.start_search(context::current(), req).await?;

if let Ok(session_id) = result {
    let mut offset = 0;
    let limit = 20;  // 每次获取 20 个
    
    loop {
        // 2. 获取结果
        let fetch_req = FetchSearchResultsRequest {
            session_id,
            offset,
            limit,
        };
        let (_, fetch_result) = client.fetch_search_results(
            context::current(), 
            fetch_req
        ).await?;
        
        if let Ok(result) = fetch_result {
            // 3. 显示结果
            for hit in &result.hits {
                println!("{:?}", hit.file_path);
            }
            
            // 4. 检查是否还有更多
            if !result.has_more {
                break;
            }
            
            // 5. 更新 offset 继续获取
            offset += result.hits.len();
            
            // 6. 如果暂时没有新结果，等待一下
            if result.hits.is_empty() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        } else {
            break;  // 错误
        }
    }
    
    // 7. 清理会话
    client.cancel_search(context::current(), session_id).await?;
}
```

---

## 会话管理

### 会话生命周期

- **创建**: 调用 `start_search` 自动创建会话
- **标识**: 使用 UUID 作为会话 ID
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
- `serve` 命令使用 `watch-paths` 作为默认索引路径
- 首次运行自动创建配置文件模板

---

## 最佳实践

### ✅ 推荐用法

1. **自然语言搜索**
   ```rust
   SearchRequest {
       query: "如何解析 JSON 文件".to_string(),
       search_mode: SearchMode::Natural,
   }
   ```

2. **规则搜索 + 过滤**
   ```rust
   SearchRequest {
       query: "*.rs size:<100KB mtime:<1w".to_string(),
       search_mode: SearchMode::Rule,
   }
   ```

3. **复合查询**
   ```rust
   SearchRequest {
       query: "tokio AND async root:/home/dev/projects".to_string(),
       search_mode: SearchMode::Rule,
   }
   ```

### ⚠️ 注意事项

1. **Natural 模式**: 使用 AI 语义理解，需要先加载 BERT 模型
2. **Rule 模式**: 支持完整的 Query DSL 语法
3. **排序**: GUI 客户端支持按文件名、时间、相关度排序

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

| 错误类型 | 原因 | 解决方法 |
|---------|------|---------|
| `SessionNotExists` | 会话 ID 不存在或已过期 | 重新发起搜索 |
| `SessionAlreadyCancelled` | 会话已被取消 | 重新发起搜索 |
| `InvalidQuery` | 查询语法错误 | 检查 Query DSL 语法 |
| `OperateOnAlreadyFailedSearch` | 操作已失败的搜索 | 重新发起搜索 |
| `Failed to connect` | 服务器未启动 | 先运行 `cargo run -p server -- serve` |

---

**文档版本**: 2.0  
**最后更新**: 2026-01-21
