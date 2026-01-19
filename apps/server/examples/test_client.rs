//! 测试客户端 - 演示新的搜索 API
//! 
//! 运行方式:
//! 1. 先启动服务: cargo run -p server -- serve
//! 2. 运行客户端: cargo run -p server --example test_client

use rpc::{WorldClient, search::{SearchRequest, SearchMode, FetchSearchResultsRequest, SearchStatus}};
use config::AppStrategy;
use tarpc::{client, context, tokio_serde::formats::Bincode};
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
    
    // ============ Rule 模式搜索（Query DSL）============
    println!("\n{}", "=".repeat(50));
    println!("=== Rule 模式搜索（Query DSL）===");
    println!("{}", "=".repeat(50));
    
    let req = SearchRequest {
        query: "*.rs AND size:>1KB".to_string(),
        search_mode: SearchMode::Rule,
    };
    
    println!("执行查询: {}", req.query);
    
    match client.start_search(context::current(), req).await? {
        Ok(session_id) => {
            println!("✓ 搜索已启动，Session ID: {}", session_id);
            
            // 轮询获取结果
            let limit = 10;
            
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                
                // 查询状态
                match client.search_status(context::current(), session_id).await? {
                    Ok(status) => {
                        match &status {
                            SearchStatus::InProgress { found_so_far } => {
                                println!("  搜索中... 已找到 {} 个结果", found_so_far);
                            }
                            SearchStatus::Completed { total_count } => {
                                println!("✓ 搜索完成，共 {} 个结果", total_count);
                                
                                // 获取结果
                                let fetch_req = FetchSearchResultsRequest {
                                    session_id,
                                    offset: 0,
                                    limit,
                                };
                                
                                match client.fetch_search_results(context::current(), fetch_req).await? {
                                    Ok(results) => {
                                        println!("\n获取结果 (offset={}, limit={}):", 0, limit);
                                        for (i, hit) in results.hits.iter().enumerate() {
                                            println!("  [{}] {:?} (score: {:?})", 
                                                     i + 1, hit.file_path, hit.score);
                                        }
                                        println!("还有更多: {}", results.has_more);
                                    }
                                    Err(e) => {
                                        println!("获取结果失败: {:?}", e);
                                    }
                                }
                                break;
                            }
                            SearchStatus::Failed(err) => {
                                println!("✗ 搜索失败: {:?}", err);
                                break;
                            }
                            SearchStatus::Cancelled => {
                                println!("✗ 搜索已取消");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        println!("查询状态失败: {:?}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ 启动搜索失败: {:?}", e);
        }
    }
    
    // ============ Natural 模式搜索（语义搜索）============
    println!("\n{}", "=".repeat(50));
    println!("=== Natural 模式搜索（语义搜索）===");
    println!("{}", "=".repeat(50));
    
    let req = SearchRequest {
        query: "find configuration files".to_string(),
        search_mode: SearchMode::Natural,
    };
    
    println!("执行查询: {}", req.query);
    
    match client.start_search(context::current(), req).await? {
        Ok(session_id) => {
            println!("✓ 搜索已启动，Session ID: {}", session_id);
            
            // 等待搜索完成
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                
                match client.search_status(context::current(), session_id).await? {
                    Ok(SearchStatus::Completed { total_count }) => {
                        println!("✓ 搜索完成，共 {} 个结果", total_count);
                        
                        // 获取前 5 个结果
                        let fetch_req = FetchSearchResultsRequest {
                            session_id,
                            offset: 0,
                            limit: 5,
                        };
                        
                        if let Ok(results) = client.fetch_search_results(context::current(), fetch_req).await? {
                            println!("\n前 5 个结果:");
                            for (i, hit) in results.hits.iter().enumerate() {
                                println!("  [{}] {:?}", i + 1, hit.file_path);
                                println!("      Score: {:?}, Size: {} bytes", hit.score, hit.file_size);
                            }
                        }
                        break;
                    }
                    Ok(SearchStatus::InProgress { found_so_far }) => {
                        println!("  搜索中... 已找到 {} 个结果", found_so_far);
                    }
                    Ok(status) => {
                        println!("搜索状态: {:?}", status);
                        break;
                    }
                    Err(e) => {
                        println!("查询状态失败: {:?}", e);
                        break;
                    }
                }
            }
            
            // 取消搜索（清理）
            let _ = client.cancel_search(context::current(), session_id).await;
        }
        Err(e) => {
            println!("✗ 启动搜索失败: {:?}", e);
        }
    }
    
    println!("\n=== 测试完成 ===");
    Ok(())
}
