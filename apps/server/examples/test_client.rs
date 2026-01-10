//! 测试客户端 - 用于测试搜索 RPC
//! 
//! 运行方式:
//! 1. 先启动服务: cargo run -p server -- serve
//! 2. 运行客户端: cargo run -p server --example test_client

use rpc::{WorldClient, search::{SearchRequest, SortMode}};
use config::AppStrategy;
use tarpc::{client, context, tokio_serde::formats::Bincode};
use std::path::PathBuf;

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
    
    // 测试搜索
    println!("\n=== 测试搜索 ===");
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
        max_results: Some(10),
    };
    
    println!("搜索请求: {:?}", req);
    let result = client.start_search(context::current(), req).await?;
    println!("搜索结果: {:?}", result);
    
    // 如果搜索成功，获取分页结果
    if let rpc::search::SearchResult::Started { session_id, total_count } = result {
        println!("\n=== 获取搜索结果 ===");
        println!("Session ID: {}, 总结果数: {}", session_id, total_count);
        
        // 获取第一页 (每页5个)
        if let Some(page1) = client.get_results_page(context::current(), session_id, 0, 5).await? {
            println!("\n第 1 页 (共 {} 页):", page1.total_pages);
            for (i, hit) in page1.hits.iter().enumerate() {
                println!("  {}. {:?} (score: {:.2})", i + 1, hit.file_path, hit.score);
                println!("     {}", hit.snippet);
            }
        }
        
        // 如果有第二页，也获取
        if total_count > 5 {
            if let Some(page2) = client.get_results_page(context::current(), session_id, 1, 5).await? {
                println!("\n第 2 页:");
                for (i, hit) in page2.hits.iter().enumerate() {
                    println!("  {}. {:?} (score: {:.2})", i + 6, hit.file_path, hit.score);
                }
            }
        }
        
        // 测试取消会话
        println!("\n=== 取消会话 ===");
        let canceled = client.cancel_search(context::current(), session_id).await?;
        println!("会话已取消: {}", canceled);
    }
    
    Ok(())
}
