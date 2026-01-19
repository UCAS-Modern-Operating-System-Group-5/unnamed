//! 交互式搜索客户端
//! 
//! 运行方式:
//! 1. 先启动服务: cargo run -p server -- serve
//! 2. 运行客户端: cargo run -p server --example interactive_client

use rpc::{WorldClient, search::{SearchRequest, SortMode, SearchStatus, StartSearchResult}};
use config::AppStrategy;
use tarpc::{client, context, tokio_serde::formats::Bincode};
use std::path::PathBuf;
use std::time::Duration;
use std::io::{self, Write};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ServerConfig {
    #[serde(default)]
    watch_paths: Vec<PathBuf>,
}

fn load_server_config() -> Option<PathBuf> {
    let strategy = config::create_strategy().ok()?;
    let config_path = strategy.config_dir().join(config::constants::SERVER_CONFIG_FILE_NAME);
    let content = std::fs::read_to_string(&config_path).ok()?;
    let config: ServerConfig = toml::from_str(&content).ok()?;
    config.watch_paths.first().cloned()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 获取 socket 路径
    let strategy = config::create_strategy()?;
    let runtime_dir = strategy.runtime_dir().unwrap_or_else(|| std::env::temp_dir().join("unnamed"));
    let socket_path = runtime_dir.join(config::constants::UNIX_SOCKET_FILE_NAME);
    
    println!("🔌 连接到服务器: {:?}", socket_path);
    
    // 连接到服务器
    let transport = tarpc::serde_transport::unix::connect(&socket_path, Bincode::default).await?;
    let client = WorldClient::new(client::Config::default(), transport).spawn();
    
    // 测试连接
    match client.ping(context::current()).await {
        Ok(response) => println!("✓ 服务器连接成功: {}\n", response),
        Err(e) => {
            eprintln!("✗ 连接失败: {}", e);
            eprintln!("请先运行: cargo run -p server -- serve");
            return Ok(());
        }
    }
    
    // 主循环
    loop {
        println!("{}", "=".repeat(60));
        println!("🔍 AI 搜索引擎 - 交互式客户端");
        println!("{}", "=".repeat(60));
        println!("请选择操作:");
        println!("  1. 新 API: 异步搜索（推荐，支持流式/大结果集）");
        println!("  2. 旧 API: 同步搜索（传统分页）");
        println!("  q. 退出");
        print!("\n请选择 [1/2/q]: ");
        io::stdout().flush()?;
        
        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        
        match choice.trim() {
            "1" => search_async(&client).await?,
            "2" => search_sync(&client).await?,
            "q" | "Q" => {
                println!("👋 再见!");
                break;
            }
            _ => println!("⚠ 无效选择，请重试\n"),
        }
    }
    
    Ok(())
}

/// 异步搜索（新 API）
async fn search_async(client: &WorldClient) -> anyhow::Result<()> {
    println!("\n{}", "-".repeat(60));
    println!("📋 配置搜索参数");
    println!("{}", "-".repeat(60));
    
    // 从配置文件读取默认搜索目录
    let default_dir = load_server_config()
        .unwrap_or_else(|| PathBuf::from("/Users/jun/Documents"));
    
    // 获取搜索目录
    print!("搜索目录 [默认: {}]: ", default_dir.display());
    io::stdout().flush()?;
    let mut dir = String::new();
    io::stdin().read_line(&mut dir)?;
    let search_dir = if dir.trim().is_empty() {
        default_dir
    } else {
        PathBuf::from(dir.trim())
    };
    
    // 选择搜索模式
    print!("搜索模式 [1=传统关键词 / 2=AI语义搜索，默认: 1]: ");
    io::stdout().flush()?;
    let mut mode = String::new();
    io::stdin().read_line(&mut mode)?;
    let use_semantic = mode.trim() == "2";
    
    // 获取搜索查询
    print!("搜索查询（必填）: ");
    io::stdout().flush()?;
    let mut query_input = String::new();
    io::stdin().read_line(&mut query_input)?;
    let query_str = query_input.trim();
    
    if query_str.is_empty() {
        println!("⚠ 查询不能为空\n");
        return Ok(());
    }
    
    // 获取文件类型过滤
    print!("文件类型过滤（如: *.rs,*.toml，留空=全部）: ");
    io::stdout().flush()?;
    let mut globs_input = String::new();
    io::stdin().read_line(&mut globs_input)?;
    let include_globs: Vec<String> = globs_input
        .trim()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect();
    
    // 获取排除目录
    print!("排除目录（如: target,node_modules，留空=无）: ");
    io::stdout().flush()?;
    let mut exclude_input = String::new();
    io::stdin().read_line(&mut exclude_input)?;
    let exclude_globs: Vec<String> = exclude_input
        .trim()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| format!("{}/**", s.trim()))
        .collect();
    
    // 获取最大结果数
    print!("最大结果数 [默认: 100]: ");
    io::stdout().flush()?;
    let mut max_str = String::new();
    io::stdin().read_line(&mut max_str)?;
    let max_results = if max_str.trim().is_empty() {
        Some(100)
    } else {
        max_str.trim().parse().ok()
    };
    
    println!("\n{}", "-".repeat(60));
    println!("🚀 开始搜索...");
    println!("{}", "-".repeat(60));
    
    // 构建搜索请求
    let (keywords_vec, semantic_vec) = if use_semantic {
        (vec![], vec![query_str.to_string()])
    } else {
        (vec![query_str.to_string()], vec![])
    };
    
    let req = SearchRequest {
        root_directories: vec![search_dir.clone()],
        regular_expressions: vec![],
        keywords: keywords_vec,
        semantic_queries: semantic_vec,
        semantic_threshold: None,
        include_globs,
        exclude_globs,
        time_accessed_range: None,
        time_created_range: None,
        time_modified_range: None,
        size_range_bytes: None,
        sort: SortMode::Relevance,
        max_results,
    };
    
    println!("📁 搜索目录: {:?}", search_dir);
    println!("🔍 搜索模式: {}", if use_semantic { "AI语义搜索" } else { "传统关键词" });
    println!("🔑 查询: {}", query_str);
    if !req.include_globs.is_empty() {
        println!("📋 包含: {:?}", req.include_globs);
    }
    if !req.exclude_globs.is_empty() {
        println!("🚫 排除: {:?}", req.exclude_globs);
    }
    println!();
    
    // 启动异步搜索
    let result = client.start_search_async(context::current(), req).await?;
    
    match result {
        StartSearchResult::Started { session_id } => {
            println!("✓ 搜索已启动，Session ID: {}\n", session_id);
            
            // 获取结果
            let mut offset = 0;
            let limit = 10;  // 每次显示 10 个
            let mut total_displayed = 0;
            
            loop {
                tokio::time::sleep(Duration::from_millis(300)).await;
                
                let fetch = client.fetch_search_results(context::current(), session_id, offset, limit).await?;
                
                if let Some(result) = fetch {
                    // 显示新结果
                    if !result.hits.is_empty() {
                        println!("📄 结果 [{}-{}]:", offset + 1, offset + result.hits.len());
                        for (i, hit) in result.hits.iter().enumerate() {
                            let num = offset + i + 1;
                            println!("  {}. {} (评分: {:.2})", num, hit.abs_file_path.display(), hit.score);
                            println!("     📝 {}", hit.snippet);
                            println!("     📊 大小: {} bytes, 修改: {:?}", hit.file_size, hit.modified_time);
                            println!();
                        }
                        total_displayed += result.hits.len();
                        offset += result.hits.len();
                    }
                    
                    // 检查状态
                    match &result.status {
                        SearchStatus::InProgress { found_so_far } if result.hits.is_empty() => {
                            print!("\r  ⏳ 搜索中... 已找到 {} 个结果", found_so_far);
                            io::stdout().flush().ok();
                            continue;
                        }
                        SearchStatus::Completed { total_count } => {
                            println!("\n✓ 搜索完成！共找到 {} 个结果，已显示 {} 个\n", total_count, total_displayed);
                            break;
                        }
                        SearchStatus::Failed(error) => {
                            println!("\n✗ 搜索失败: {}\n", error);
                            break;
                        }
                        SearchStatus::Cancelled => {
                            println!("\n⚠ 搜索已取消\n");
                            break;
                        }
                        _ => {}
                    }
                    
                    // 如果还有更多，询问是否继续
                    if result.has_more && !result.hits.is_empty() {
                        print!("继续显示更多结果? [y/n]: ");
                        io::stdout().flush()?;
                        let mut cont = String::new();
                        io::stdin().read_line(&mut cont)?;
                        
                        if cont.trim().to_lowercase() != "y" {
                            println!("已停止显示，但搜索仍在后台进行...");
                            // 取消搜索
                            client.cancel_search(context::current(), session_id).await?;
                            println!("✓ 搜索已取消\n");
                            break;
                        }
                    }
                } else {
                    println!("✗ 会话不存在或已过期\n");
                    break;
                }
            }
        }
        StartSearchResult::Failed(error) => {
            println!("✗ 搜索启动失败: {}\n", error);
        }
    }
    
    Ok(())
}

/// 同步搜索（旧 API）
async fn search_sync(client: &WorldClient) -> anyhow::Result<()> {
    println!("\n{}", "-".repeat(60));
    println!("📋 配置搜索参数（同步模式）");
    println!("{}", "-".repeat(60));
    
    // 从配置文件读取默认搜索目录
    let default_dir = load_server_config()
        .unwrap_or_else(|| PathBuf::from("/Users/jun/Documents"));
    
    // 获取搜索目录
    print!("搜索目录 [默认: {}]: ", default_dir.display());
    io::stdout().flush()?;
    let mut dir = String::new();
    io::stdin().read_line(&mut dir)?;
    let search_dir = if dir.trim().is_empty() {
        default_dir
    } else {
        PathBuf::from(dir.trim())
    };
    
    // 选择搜索模式
    print!("搜索模式 [1=传统关键词 / 2=AI语义搜索，默认: 1]: ");
    io::stdout().flush()?;
    let mut mode = String::new();
    io::stdin().read_line(&mut mode)?;
    let use_semantic = mode.trim() == "2";
    
    // 获取搜索查询
    print!("搜索查询（必填）: ");
    io::stdout().flush()?;
    let mut query_input = String::new();
    io::stdin().read_line(&mut query_input)?;
    let query_str = query_input.trim();
    
    if query_str.is_empty() {
        println!("⚠ 查询不能为空\n");
        return Ok(());
    }
    
    // 获取最大结果数
    print!("最大结果数 [默认: 50]: ");
    io::stdout().flush()?;
    let mut max_str = String::new();
    io::stdin().read_line(&mut max_str)?;
    let max_results = if max_str.trim().is_empty() {
        Some(50)
    } else {
        max_str.trim().parse().ok()
    };
    
    println!("\n{}", "-".repeat(60));
    println!("🚀 开始搜索（请稍候，等待搜索完成...）");
    println!("📝 搜索模式: {}", if use_semantic { "AI语义搜索" } else { "传统关键词" });
    println!("🔍 查询: {}", query_str);
    println!("{}", "-".repeat(60));
    
    // 构建搜索请求（根据模式选择填充 keywords 或 semantic_queries）
    let (keywords_vec, semantic_vec) = if use_semantic {
        (vec![], vec![query_str.to_string()])
    } else {
        (vec![query_str.to_string()], vec![])
    };
    
    let req = SearchRequest {
        root_directories: vec![search_dir.clone()],
        regular_expressions: vec![],
        keywords: keywords_vec,
        semantic_queries: semantic_vec,
        semantic_threshold: None,
        include_globs: vec![],
        exclude_globs: vec![],
        time_accessed_range: None,
        time_created_range: None,
        time_modified_range: None,
        size_range_bytes: None,
        sort: SortMode::Relevance,
        max_results,
    };
    
    // 同步搜索（会阻塞）
    let result = client.start_search(context::current(), req).await?;
    
    match result {
        rpc::search::SearchResult::Started { session_id, total_count } => {
            println!("✓ 搜索完成！共找到 {} 个结果\n", total_count);
            
            if total_count == 0 {
                println!("没有找到匹配的结果\n");
                return Ok(());
            }
            
            // 分页显示
            let page_size = 10;
            let total_pages = (total_count + page_size - 1) / page_size;
            let mut current_page = 0;
            
            loop {
                if let Some(page_result) = client.get_results_page(
                    context::current(),
                    session_id,
                    current_page,
                    page_size
                ).await? {
                    println!("📄 第 {} 页 / 共 {} 页:", current_page + 1, total_pages);
                    println!("{}", "-".repeat(60));
                    
                    for (i, hit) in page_result.hits.iter().enumerate() {
                        let num = current_page * page_size + i + 1;
                        println!("{}. {} (评分: {:.2})", num, hit.file_path.display(), hit.score);
                        println!("   📝 {}", hit.snippet);
                        println!();
                    }
                    
                    // 询问是否继续
                    if current_page + 1 < total_pages {
                        print!("下一页 [n] | 上一页 [p] | 跳转 [数字] | 退出 [q]: ");
                        io::stdout().flush()?;
                        let mut action = String::new();
                        io::stdin().read_line(&mut action)?;
                        
                        match action.trim() {
                            "n" | "N" | "" => current_page += 1,
                            "p" | "P" => {
                                if current_page > 0 {
                                    current_page -= 1;
                                } else {
                                    println!("已经是第一页了");
                                }
                            }
                            "q" | "Q" => break,
                            num_str => {
                                if let Ok(page_num) = num_str.parse::<usize>() {
                                    if page_num > 0 && page_num <= total_pages {
                                        current_page = page_num - 1;
                                    } else {
                                        println!("⚠ 页码超出范围 (1-{})", total_pages);
                                    }
                                } else {
                                    println!("⚠ 无效输入");
                                }
                            }
                        }
                    } else {
                        println!("已显示全部结果");
                        break;
                    }
                } else {
                    println!("✗ 获取结果失败\n");
                    break;
                }
            }
            
            // 清理会话
            client.cancel_search(context::current(), session_id).await?;
        }
        rpc::search::SearchResult::Failed(error) => {
            println!("✗ 搜索失败: {}\n", error);
        }
    }
    
    Ok(())
}
