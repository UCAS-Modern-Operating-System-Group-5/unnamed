//! 测试客户端 - 演示新旧两种搜索 API
//! 
//! 运行方式:
//! 1. 先启动服务: cargo run -p server -- serve
//! 2. 运行客户端: cargo run -p server --example test_client

use rpc::{WorldClient, search::{SearchRequest, SortMode, SearchStatus}};
use config::AppStrategy;
use tarpc::{client, context, tokio_serde::formats::Bincode};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 获取 socket 路径
    let strategy = config::create_strategy()?;
    let runtime_dir = strategy.runtime_dir().unwrap_or_else(|| std::env::temp_dir().join("unnamed"));
    let socket_path = runtime_dir.join(config::constants::UNIX_SOCKET_FILE_NAME);
    
    println!("连接到: {:?}", socket_path);
    
    // 连接到服务器
    let transport = tarpc::serde_transport::unix::connect(&socket_path, Bincode::default).await?;
    let client = WorldClient::new(client::Config::default(), transport).spawn();
    
    // 测试 ping
    println!("\n=== 测试 ping ===");
    let response = client.ping(context::current()).await?;
    println!("Ping 响应: {}", response);
    
    // 构建搜索请求
    let req = SearchRequest {
        root_directories: vec![PathBuf::from("/Users/jun/Documents/code/rust/ai_search_demo/docs")],
        regular_expressions: vec![],
        keywords: vec!["test".to_string()],
        semantic_queries: vec![],
        semantic_threshold: None,
        include_globs: vec![],
        exclude_globs: vec![],
        time_accessed_range: None,
        time_created_range: None,
        time_modified_range: None,
        size_range_bytes: None,
        sort: SortMode::Relevance,
        max_results: Some(100),
    };
    
    // ============ 新 API: 异步 + Offset-based ============
    println!("\n{}", "=".repeat(50));
    println!("=== 新 API: 异步搜索 + Offset-based 分页 ===");
    println!("{}", "=".repeat(50));
    
    let req2 = SearchRequest {
        root_directories: vec![PathBuf::from("/Users/jun/Documents/code/rust/ai_search_demo/docs")],
        regular_expressions: vec![],
        keywords: vec!["test".to_string()],
        semantic_queries: vec![],
        semantic_threshold: None,
        include_globs: vec![],
        exclude_globs: vec![],
        time_accessed_range: None,
        time_created_range: None,
        time_modified_range: None,
        size_range_bytes: None,
        sort: SortMode::Relevance,
        max_results: Some(100),
    };
    
    let result = client.start_search_async(context::current(), req2).await?;
    
    if let rpc::search::StartSearchResult::Started { session_id } = result {
        println!("✓ 搜索已启动，Session ID: {}", session_id);
        
        // 无限滚动模式获取结果
        let mut offset = 0;
        let limit = 3;  // 每次获取 3 个
        let mut last_found = 0;  // 上次找到的数量
        let mut poll_count = 0;  // 轮询次数
        const MAX_POLLS: usize = 50;  // 最大轮询次数，防止无限循环
        
        println!("  等待搜索结果...");
        
        loop {
            poll_count += 1;
            if poll_count > MAX_POLLS {
                println!("⚠ 达到最大轮询次数，退出");
                break;
            }
            
            let fetch = client.fetch_results(context::current(), session_id, offset, limit).await?;
            
            if let Some(result) = fetch {
                // 只在有新结果或状态变化时打印
                let current_found = match &result.status {
                    SearchStatus::InProgress { found_so_far } => *found_so_far,
                    SearchStatus::Completed { total_count } => *total_count,
                    _ => 0,
                };
                
                let has_new_results = !result.hits.is_empty();
                let status_changed = current_found != last_found;
                
                if has_new_results || !result.has_more {
                    // 有新结果或搜索完成时打印
                    println!("\n--- 获取结果 [offset={}, limit={}] ---", offset, limit);
                    println!("状态: {:?}", result.status);
                    println!("本次返回: {} 个, has_more: {}", result.hits.len(), result.has_more);
                    
                    for (i, hit) in result.hits.iter().enumerate() {
                        println!("  {}. {:?} (score: {:.2})", offset + i + 1, hit.file_path, hit.score);
                    }
                } else if status_changed {
                    // 状态变化但没有新结果时简单提示
                    print!("\r  搜索中... 已找到 {} 个结果", current_found);
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                }
                
                last_found = current_found;
                
                // 检查是否还有更多
                if !result.has_more {
                    // 打印最终状态
                    if let SearchStatus::Completed { total_count } = result.status {
                        println!("\n✓ 搜索完成，总共 {} 个结果", total_count);
                    }
                    break;
                }
                
                // 移动 offset（只有有新结果时才移动）
                if has_new_results {
                    offset += result.hits.len();
                }
                
                // 等待一段时间再轮询（500ms，减少无意义输出）
                tokio::time::sleep(Duration::from_millis(500)).await;
            } else {
                println!("会话不存在或已过期");
                break;
            }
        }
        
        // 取消会话
        let canceled = client.cancel_search(context::current(), session_id).await?;
        println!("\n会话已清理: {}", canceled);
    } else {
        println!("搜索启动失败: {:?}", result);
    }
    
    // ============ 旧 API: 同步 + Page-based ============
    println!("\n{}", "=".repeat(50));
    println!("=== 旧 API: 同步搜索 + Page-based 分页 ===");
    println!("{}", "=".repeat(50));
    
    let result = client.start_search(context::current(), req).await?;
    
    if let rpc::search::SearchResult::Started { session_id, total_count } = result {
        println!("✓ 搜索完成，Session ID: {}, 总结果数: {}", session_id, total_count);
        
        // 获取第一页
        if let Some(page1) = client.get_results_page(context::current(), session_id, 0, 5).await? {
            println!("\n第 1 页 (共 {} 页):", page1.total_pages);
            for (i, hit) in page1.hits.iter().enumerate() {
                println!("  {}. {:?} (score: {:.2})", i + 1, hit.file_path, hit.score);
            }
        }
        
        let canceled = client.cancel_search(context::current(), session_id).await?;
        println!("\n会话已清理: {}", canceled);
    }
    
    println!("\n=== 测试完成 ===");
    
    Ok(())
}
