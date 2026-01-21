# Unnamed 项目架构文档

本文档详细描述项目的代码组织、模块依赖关系和核心架构。

---

## 📁 项目目录结构

```text
unnamed/
├── apps/                      # 应用层（可执行程序）
│   ├── server/               # 搜索服务器
│   │   ├── src/
│   │   │   ├── main.rs       # 入口：CLI 解析 + 命令分发
│   │   │   ├── cli.rs        # Clap 命令行定义
│   │   │   ├── config.rs     # 配置加载（server.toml）
│   │   │   ├── session.rs    # 会话管理器
│   │   │   ├── command/      # 命令实现
│   │   │   │   ├── mod.rs    # Command trait 定义
│   │   │   │   ├── serve.rs  # serve 命令：启动 RPC 服务
│   │   │   │   └── index.rs  # index 命令：建立索引
│   │   │   └── indexer/      # 索引辅助模块
│   │   └── examples/         # 示例客户端
│   │       ├── test_client.rs
│   │       └── interactive_client.rs
│   └── gui/                  # GUI 客户端（未完成）
│
├── crates/                   # 核心库（可复用）
│   ├── search-core/          # 搜索引擎核心
│   │   └── src/
│   │       ├── lib.rs        # 库入口 + SearchEngine 结构
│   │       ├── ai.rs         # BERT 模型封装
│   │       ├── cache.rs      # sled Embedding 缓存
│   │       ├── indexer.rs    # 索引构建与监控
│   │       ├── search.rs     # 搜索执行
│   │       ├── extract.rs    # 文本提取（PDF/TXT）
│   │       ├── registry.rs   # 文件处理协调器
│   │       ├── rpc_compat.rs # RPC 类型适配层
│   │       ├── models.rs     # 数据模型
│   │       └── config.rs     # 配置结构
│   ├── rpc/                  # RPC 接口定义
│   │   └── src/
│   │       ├── lib.rs        # tarpc 服务定义
│   │       └── search.rs     # 搜索相关类型
│   └── config/               # 配置管理
│
└── docs/                     # 文档
    ├── API_REFERENCE.md      # API 接口文档
    ├── ARCHITECTURE.md       # 本文档
    └── USAGE.md              # 使用指南
```

---

## 🏗️ 分层架构

```mermaid
graph TB
    subgraph "应用层 (apps/)"
        CLI[CLI 入口<br/>main.rs]
        Serve[Serve 命令<br/>serve.rs]
        Index[Index 命令<br/>index.rs]
        Session[会话管理器<br/>session.rs]
        Client[客户端示例<br/>examples/]
    end

    subgraph "RPC 层 (crates/rpc/)"
        RPCDef[tarpc 服务定义<br/>lib.rs]
        Types[搜索类型<br/>search.rs]
    end

    subgraph "核心层 (crates/search-core/)"
        Engine[SearchEngine<br/>lib.rs]
        AI[AI 模块<br/>ai.rs]
        Cache[缓存模块<br/>cache.rs]
        Indexer[索引模块<br/>indexer.rs]
        Search[搜索模块<br/>search.rs]
        Extract[提取模块<br/>extract.rs]
        Registry[协调器<br/>registry.rs]
        Compat[RPC 适配<br/>rpc_compat.rs]
    end

    subgraph "外部依赖"
        Tantivy[Tantivy<br/>倒排索引]
        Candle[Candle<br/>BERT 推理]
        Sled[Sled<br/>KV 缓存]
        Tarpc[tarpc<br/>RPC 框架]
        Notify[notify<br/>文件监控]
    end

    CLI --> Serve
    CLI --> Index
    Serve --> Session
    Serve --> RPCDef
    Serve --> Compat
    Session --> Types
    Client --> RPCDef

    RPCDef --> Types
    Compat --> Engine
    Compat --> Types

    Engine --> AI
    Engine --> Cache
    Engine --> Indexer
    Engine --> Search
    Engine --> Registry

    Indexer --> Extract
    Indexer --> Registry
    Indexer --> Notify

    AI --> Candle
    Cache --> Sled
    Search --> Tantivy
    Indexer --> Tantivy
```

---

## 🔄 数据流架构

### 索引流程

```mermaid
sequenceDiagram
    participant User as 用户
    participant CLI as main.rs
    participant Index as IndexCommand
    participant Engine as SearchEngine
    participant Indexer as indexer.rs
    participant AI as ai.rs
    participant Cache as cache.rs
    participant Tantivy as Tantivy Index

    User->>CLI: cargo run -- index /docs
    CLI->>Index: execute()
    Index->>Engine: 初始化
    Engine->>AI: 加载 BERT 模型
    Engine->>Cache: 加载 Embedding 缓存
    Engine->>Tantivy: 打开/创建索引

    Index->>Indexer: scan_existing_files()
    loop 每个文件
        Indexer->>Cache: check_file_status()
        alt 文件未变化
            Indexer->>Indexer: 跳过
        else 新增或修改
            Indexer->>Indexer: extract_text()
            Indexer->>AI: extract_keywords()
            AI->>Cache: 存入缓存
            Indexer->>Tantivy: add_document()
        end
    end
    Indexer->>Tantivy: commit()
    Index-->>User: 索引完成
```

### 搜索流程（异步）

```mermaid
sequenceDiagram
    participant Client as 客户端
    participant RPC as tarpc Server
    participant Serve as serve.rs
    participant Session as SessionManager
    participant Compat as rpc_compat.rs
    participant Engine as SearchEngine
    participant Search as search.rs

    Client->>RPC: start_search(req)
    RPC->>Serve: World::start_search()
    Serve->>Session: create_async_session()
    Session-->>Serve: session_id (UUID)
    Serve-->>Client: Ok(session_id)
    Note over Client: 立即返回，不阻塞

    Serve->>Serve: tokio::spawn()
    activate Serve
    Serve->>Compat: search_sync(engine, req)
    Compat->>Engine: 构建查询
    Engine->>Search: search_index()
    Search-->>Compat: results
    Compat-->>Serve: Vec<SearchResultItem>
    Serve->>Session: append_results()
    Serve->>Session: mark_completed()
    deactivate Serve

    loop 轮询获取结果
        Client->>RPC: fetch_search_results(req)
        RPC->>Serve: World::fetch_search_results()
        Serve->>Session: fetch_results()
        Session-->>Client: FetchResults { hits, has_more }
        alt has_more == false
            Client->>Client: 停止轮询
        end
    end

    Client->>RPC: cancel_search(session_id)
    RPC->>Session: cancel_session()
```

---

## 📦 模块依赖关系

### Crate 依赖图

```mermaid
graph LR
    subgraph "apps"
        Server[server]
        GUI[gui]
    end

    subgraph "crates"
        RPC[rpc]
        Core[search-core]
        Config[config]
    end

    Server --> RPC
    Server --> Core
    Server --> Config
    GUI --> RPC
    Core -.->|"feature: rpc-compat"| RPC
```

### search-core 内部依赖

```mermaid
graph TD
    subgraph "search-core/src/"
        Lib[lib.rs<br/>SearchEngine]
        AI[ai.rs<br/>BertModel]
        Cache[cache.rs<br/>EmbeddingCache]
        Indexer[indexer.rs<br/>索引构建]
        Search[search.rs<br/>搜索执行]
        Extract[extract.rs<br/>文本提取]
        Registry[registry.rs<br/>FileRegistry]
        Models[models.rs<br/>FileDoc]
        Config[config.rs<br/>配置结构]
        Compat[rpc_compat.rs<br/>RPC 适配]
    end

    Lib --> AI
    Lib --> Cache
    Lib --> Indexer
    Lib --> Search
    Lib --> Registry
    Lib --> Config
    Lib --> Models

    Indexer --> Extract
    Indexer --> Registry
    Indexer --> Cache
    Indexer --> AI

    Search --> AI

    Compat --> Lib
    Compat --> Search
```

### server 内部依赖

```mermaid
graph TD
    subgraph "server/src/"
        Main[main.rs]
        CLI[cli.rs]
        SrvConfig[config.rs]
        Session[session.rs]
        Error[error.rs]
        
        subgraph "command/"
            Mod[mod.rs<br/>Command trait]
            Serve[serve.rs]
            Index[index.rs]
        end
    end

    Main --> CLI
    Main --> SrvConfig
    Main --> Mod
    
    Mod --> Serve
    Mod --> Index
    
    Serve --> Session
    Serve --> SrvConfig
    Index --> SrvConfig

    Main --> Error
    Serve --> Error
    Index --> Error
```

---

## 🔧 核心组件详解

### 1. SearchEngine (`search-core/src/lib.rs`)

搜索引擎的统一入口，聚合所有核心组件：

```rust
pub struct SearchEngine {
    pub index: tantivy::Index,       // Tantivy 索引实例
    pub schema: tantivy::Schema,     // 索引 Schema
    pub reader: tantivy::IndexReader,// 索引读取器
    pub bert: BertModel,             // BERT 模型
    pub cache: EmbeddingCache,       // Embedding 缓存
    pub registry: FileRegistry,      // 文件处理协调器
    pub config: SearchConfig,        // 搜索配置
}
```

**职责**:
- 初始化所有子系统
- 提供统一的搜索接口
- 管理资源生命周期

### 2. SessionManager (`server/src/session.rs`)

管理搜索会话，支持两种模式：

```mermaid
stateDiagram-v2
    [*] --> InProgress: create_async_session()
    InProgress --> InProgress: append_results()
    InProgress --> Completed: mark_completed()
    InProgress --> Failed: mark_failed()
    InProgress --> Cancelled: cancel_session()
    Completed --> [*]: 超时清理
    Failed --> [*]: 超时清理
    Cancelled --> [*]: 超时清理
```

**API**:
| 方法 | 说明 |
|------|------|
| `create_session(hits)` | 同步模式：直接传入所有结果 |
| `create_async_session()` | 异步模式：创建空会话 |
| `append_results(id, hits)` | 追加结果（异步模式） |
| `mark_completed(id)` | 标记完成 |
| `fetch_results(id, offset, limit)` | 获取结果（offset-based） |
| `get_page(id, page, size)` | 获取分页（page-based） |
| `cancel_session(id)` | 取消会话 |

### 3. rpc_compat (`search-core/src/rpc_compat.rs`)

RPC 类型适配层，桥接 `rpc` crate 和 `search-core`：

```mermaid
graph LR
    RPCTypes[rpc::SearchRequest] --> Compat[rpc_compat.rs]
    Compat --> CoreTypes[search_core::SearchHit]
    Compat --> Filter[应用过滤器]
    Filter --> Result[SearchResultItem]
```

**关键函数**:
```rust
// 同步搜索（内部调用 Tantivy）
pub fn search_sync(engine: &SearchEngine, req: &RpcSearchRequest) 
    -> Result<Vec<SearchResultItem>, String>

// 应用 root_directories 过滤
filtered.retain(|item| {
    req.root_directories.iter().any(|root| {
        item.path.starts_with(root)
    })
});
```

### 4. FileRegistry (`search-core/src/registry.rs`)

防止扫描和监听线程重复处理同一文件：

```mermaid
graph TD
    Scanner[扫描线程] --> Registry{FileRegistry}
    Watcher[监听线程] --> Registry
    
    Registry -->|"try_start_processing()"| Lock[原子获取处理权]
    Lock -->|成功| Process[处理文件]
    Lock -->|失败| Skip[跳过]
    Process --> Finish["finish_processing()"]
```

### 5. EmbeddingCache (`search-core/src/cache.rs`)

基于 sled 的双重缓存：

```mermaid
graph TD
    subgraph "EmbeddingCache"
        EC[Embedding 缓存<br/>key: 文件路径<br/>value: 关键词列表]
        MC[元数据缓存<br/>key: 文件路径<br/>value: size + mtime]
    end
    
    Check{检查缓存} --> EC
    EC -->|命中| UseKeywords[使用缓存关键词]
    EC -->|未命中| Compute[BERT 计算]
    Compute --> Store[存入缓存]
    
    CheckMeta{检查元数据} --> MC
    MC -->|未变化| SkipFile[跳过文件]
    MC -->|已变化| ProcessFile[处理文件]
```

---

## 🌐 RPC 服务定义

```rust
#[tarpc::service]
pub trait World {
    // 健康检查
    async fn ping() -> String;

    // ===== 新 API（异步流式）=====
    async fn start_search_async(req: SearchRequest) -> StartSearchResult;
    async fn fetch_results(session_id: usize, offset: usize, limit: usize) -> Option<FetchResults>;
    async fn cancel_search(session_id: usize) -> bool;

    // ===== 旧 API（同步分页）=====
    async fn start_search(req: SearchRequest) -> SearchResult;
    async fn get_results_page(session_id: usize, page: usize, page_size: usize) -> Option<PagedResults>;
}
```

### 类型关系

```mermaid
classDiagram
    class SearchRequest {
        +Vec~PathBuf~ root_directories
        +Vec~String~ keywords
        +Vec~String~ semantic_queries
        +Vec~String~ include_globs
        +Vec~String~ exclude_globs
        +Option~usize~ max_results
        +SortMode sort
    }

    class StartSearchResult {
        <<enum>>
        Started(session_id)
        Failed(String)
    }

    class SearchStatus {
        <<enum>>
        InProgress(found_so_far)
        Completed(total_count)
        Failed(String)
        Cancelled
    }

    class FetchResults {
        +usize session_id
        +usize offset
        +Vec~SearchHit~ hits
        +SearchStatus status
        +bool has_more
    }

    class SearchHit {
        +PathBuf file_path
        +f32 score
        +String snippet
        +u64 file_size
        +SystemTime modified_time
    }

    SearchRequest --> StartSearchResult : start_search_async
    StartSearchResult --> FetchResults : fetch_results
    FetchResults --> SearchStatus
    FetchResults --> SearchHit
```

---

## 🔀 新旧 API 对比

| 特性 | 新 API (Offset-based) | 旧 API (Page-based) |
|------|----------------------|---------------------|
| 启动方法 | `start_search_async()` | `start_search()` |
| 返回时机 | 立即返回 | 等待搜索完成 |
| 获取结果 | `fetch_results(offset, limit)` | `get_results_page(page, size)` |
| 是否知道总数 | 搜索完成后才知道 | 启动时就知道 |
| 适用场景 | 流式/无限滚动/大数据集 | 传统分页/小数据集 |
| 会话状态 | InProgress → Completed | 直接 Completed |

---

## 🚀 启动流程

```mermaid
flowchart TD
    Start([cargo run -p server -- serve]) --> LoadConfig[加载配置<br/>config.rs]
    LoadConfig --> InitEngine[初始化 SearchEngine]
    
    subgraph InitEngine
        LoadModel[加载 BERT 模型] --> LoadCache[加载 Embedding 缓存]
        LoadCache --> OpenIndex[打开 Tantivy 索引]
        OpenIndex --> CreateReader[创建 IndexReader]
    end
    
    InitEngine --> CreateSession[创建 SessionManager]
    CreateSession --> CreateServer[创建 RPC Server]
    CreateServer --> BindSocket[绑定 Unix Socket]
    BindSocket --> Listen[开始监听]
    
    Listen --> Accept{接收连接}
    Accept --> Spawn[tokio::spawn 处理]
    Spawn --> Accept
```

---

## 📊 技术栈总结

| 层级 | 组件 | 技术 | 用途 |
|------|------|------|------|
| **应用层** | server | clap + tokio | CLI + 异步运行时 |
| **RPC 层** | rpc | tarpc + bincode | 高性能 RPC |
| **搜索层** | search-core | tantivy + tantivy-jieba | 倒排索引 + 中文分词 |
| **AI 层** | ai.rs | candle | BERT 推理 |
| **缓存层** | cache.rs | sled + bincode | 嵌入式 KV |
| **监控层** | indexer.rs | notify | 文件系统事件 |
| **提取层** | extract.rs | pdf-extract | PDF 文本提取 |

---

## 📝 扩展指南

### 添加新的 RPC 方法

1. **定义接口** (`crates/rpc/src/lib.rs`):
```rust
#[tarpc::service]
pub trait World {
    // 添加新方法
    async fn new_method(param: Type) -> ReturnType;
}
```

2. **实现接口** (`apps/server/src/command/serve.rs`):
```rust
impl World for Server {
    async fn new_method(self, _c: Context, param: Type) -> ReturnType {
        // 实现逻辑
    }
}
```

### 添加新的搜索过滤器

1. **扩展 SearchRequest** (`crates/rpc/src/search.rs`):
```rust
pub struct SearchRequest {
    pub new_filter: Option<NewFilterType>,
    // ...
}
```

2. **实现过滤** (`crates/search-core/src/rpc_compat.rs`):
```rust
if let Some(filter) = &req.new_filter {
    filtered.retain(|item| apply_filter(item, filter));
}
```

---

**文档版本**: 1.0  
**最后更新**: 2026-01-12
