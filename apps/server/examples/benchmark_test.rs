//! Benchmark 测试 - 自动化批量测试搜索准确率
//! 
//! 运行方式:
//! cargo run -p server --example benchmark_test           # 测试中文数据集（默认）
//! cargo run -p server --example benchmark_test -- --lang ZH  # 指定中文数据集
//! cargo run -p server --example benchmark_test -- --lang EN  # 指定英文数据集
//! cargo run -p server --example benchmark_test -- --limit 10  # 只测试前10个文件（debug模式）
//! cargo run -p server --example benchmark_test -- --lang EN --limit 5  # 英文数据集，测试前5个
//! 
//! 功能：
//! 1. 备份原有索引
//! 2. 自动执行索引（记录索引时间）
//! 3. 自动启动 server
//! 4. 批量测试搜索准确率
//! 5. 生成 result.csv（详细结果）
//! 6. 生成 report.txt（总结报告）
//! 7. 恢复原有索引
//!
//! Debug 模式 (--limit N):
//! - 只拷贝前 N 个文件到临时 test 文件夹
//! - 使用临时文件夹进行索引和测试
//! - 测试完成后自动删除临时文件夹

use rpc::{WorldClient, search::{SearchRequest, SearchMode, FetchSearchResultsRequest, SearchStatus}};
use config::AppStrategy;
use tarpc::{client, context, tokio_serde::formats::Bincode};
use std::time::{Duration, Instant};
use std::fs::{self, File};
use std::io::{BufReader, BufRead, Write};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use std::process::Stdio;
use chrono;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;

#[derive(Debug)]
struct TestCase {
    question: String,
    // ZH: title（单个答案）; EN: expected_files（多个答案）
    title: Option<String>,
    expected_files: Vec<String>,  // EN 用，存储所有预期答案
}

#[derive(Debug)]
struct TestResult {
    question: String,
    expected: String,  // 改为通用的 expected，可以是 title 或 keyword
    found: bool,
    rank: Option<usize>, // 如果找到，记录排名位置
    total_results: usize,
    search_time_ms: u64,
}

/// 读取 ZH 的 keyword_index.json 文件
fn load_test_cases_zh(json_path: &Path) -> anyhow::Result<Vec<TestCase>> {
    let content = std::fs::read_to_string(json_path)?;
    let keyword_index: HashMap<String, Vec<String>> = serde_json::from_str(&content)?;
    
    let mut cases = Vec::new();
    for (keyword, titles) in keyword_index.iter() {
        if !keyword.is_empty() && !titles.is_empty() {
            cases.push(TestCase {
                question: keyword.clone(),
                title: Some(titles[0].clone()),  // ZH 只用第一个 title
                expected_files: titles.clone(),   // 但也存储所有的（为了兼容）
            });
        }
    }
    
    // 按关键词排序以保证顺序一致
    cases.sort_by(|a, b| a.question.cmp(&b.question));
    
    Ok(cases)
}

/// 读取 ZH 的旧 card.csv 文件（兼容旧格式）
fn load_test_cases_zh_csv(csv_path: &Path) -> anyhow::Result<Vec<TestCase>> {
    let file = File::open(csv_path)?;
    let reader = BufReader::new(file);
    let mut cases = Vec::new();
    
    for (idx, line) in reader.lines().enumerate() {
        // 跳过表头
        if idx == 0 {
            continue;
        }
        
        let line = line?;
        let parts: Vec<&str> = line.split(',').collect();
        
        if parts.len() >= 2 {
            let title = parts[0].to_string();
            let question = parts[1].to_string();
            
            // 跳过空问题
            if !question.is_empty() && !title.is_empty() {
                cases.push(TestCase { 
                    question, 
                    title: Some(title.clone()),
                    expected_files: vec![title],  // ZH 不使用多文件
                });
            }
        }
    }
    
    Ok(cases)
}

/// 读取 EN 的 keyword_index.json 文件
fn load_test_cases_en(json_path: &Path) -> anyhow::Result<Vec<TestCase>> {
    let content = std::fs::read_to_string(json_path)?;
    let keyword_index: HashMap<String, Vec<String>> = serde_json::from_str(&content)?;
    
    let mut cases = Vec::new();
    for (keyword, files) in keyword_index.iter() {
        if !keyword.is_empty() && !files.is_empty() {
            cases.push(TestCase {
                question: keyword.clone(),
                title: None,  // EN 不使用 title
                expected_files: files.clone(),
            });
        }
    }
    
    // 按关键词排序以保证顺序一致
    cases.sort_by(|a, b| a.question.cmp(&b.question));
    
    Ok(cases)
}

/// 通用的读取测试用例函数（都使用 JSON 格式）
fn load_test_cases(lang_dir: &Path, lang: &str) -> anyhow::Result<Vec<TestCase>> {
    // EN 的 keyword_index.json 在 processed 目录下
    let json_path = if lang == "EN" {
        lang_dir.join("processed").join("keyword_index.json")
    } else {
        lang_dir.join("keyword_index.json")
    };
    
    if !json_path.exists() {
        // 如果 JSON 不存在，尝试从 CSV 读取（仅用于兼容）
        println!("⚠️  keyword_index.json 不存在，尝试从 card.csv 读取");
        let csv_path = lang_dir.join("card.csv");
        if lang == "EN" {
            load_test_cases_zh_csv(&csv_path)  // 两种格式都用同一个 CSV 读取函数
        } else {
            load_test_cases_zh_csv(&csv_path)
        }
    } else {
        // 两种语言都从 JSON 读取
        if lang == "EN" {
            load_test_cases_en(&json_path)
        } else {
            load_test_cases_zh(&json_path)
        }
    }
}

/// 获取索引目录路径
fn get_index_dir() -> anyhow::Result<PathBuf> {
    let strategy = config::create_strategy()?;
    let cache_dir = strategy.cache_dir();
    Ok(cache_dir.join("index"))
}

/// 获取 embedding cache 目录路径
fn get_embedding_cache_dir() -> anyhow::Result<PathBuf> {
    let strategy = config::create_strategy()?;
    let cache_dir = strategy.cache_dir();
    Ok(cache_dir.join("embedding_cache"))
}

/// 准备测试数据：如果指定了 limit，则拷贝前 N 个文件到临时目录
fn prepare_test_data(source_dir: &Path, limit: Option<usize>) -> anyhow::Result<(PathBuf, Option<PathBuf>)> {
    if let Some(n) = limit {
        // Debug 模式：拷贝前 N 个文件到临时目录
        let test_dir = source_dir.parent().unwrap_or(Path::new(".")).join("test_temp");
        
        // 清理旧的临时目录
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir)?;
        }
        fs::create_dir_all(&test_dir)?;
        
        println!("📁 Debug 模式: 拷贝前 {} 个文件到临时目录 {:?}", n, test_dir);
        
        // 获取源目录中的文件，按文件名排序
        let mut files: Vec<_> = fs::read_dir(source_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .collect();
        files.sort_by_key(|e| e.file_name());
        
        // 拷贝前 N 个文件
        let copy_count = std::cmp::min(n, files.len());
        for entry in files.iter().take(copy_count) {
            let src_path = entry.path();
            let dst_path = test_dir.join(entry.file_name());
            fs::copy(&src_path, &dst_path)?;
        }
        
        println!("✓ 已拷贝 {} 个文件到临时目录", copy_count);
        
        Ok((test_dir.clone(), Some(test_dir)))
    } else {
        // 正常模式：直接使用源目录
        Ok((source_dir.to_path_buf(), None))
    }
}

/// 清理临时测试目录
fn cleanup_test_data(temp_dir: Option<PathBuf>) -> anyhow::Result<()> {
    if let Some(dir) = temp_dir {
        if dir.exists() {
            println!("🗑️  删除临时测试目录: {:?}", dir);
            fs::remove_dir_all(&dir)?;
            println!("✓ 临时目录已删除");
        }
    }
    Ok(())
}

/// 杀掉可能存在的旧 server 进程
async fn kill_existing_server() -> anyhow::Result<()> {
    println!("🔍 检查是否有旧 server 进程...");
    
    // 尝试查找并杀掉 server 进程
    let output = Command::new("pkill")
        .args(&["-f", "target/debug/server serve"])
        .output()
        .await;
    
    // pkill 返回非零不代表错误，可能只是没找到进程
    if let Ok(out) = output {
        if out.status.success() {
            println!("✓ 已杀掉旧 server 进程");
            // 等待进程完全退出
            tokio::time::sleep(Duration::from_secs(1)).await;
        } else {
            println!("ℹ️  未发现运行中的 server 进程");
        }
    }
    
    Ok(())
}

/// 备份数据（索引 + embedding cache）
fn backup_data() -> anyhow::Result<(Option<PathBuf>, Option<PathBuf>)> {
    let index_dir = get_index_dir()?;
    let cache_dir = get_embedding_cache_dir()?;
    
    let mut index_backup = None;
    let mut cache_backup = None;
    
    // 备份索引
    if index_dir.exists() {
        let backup_dir = index_dir.with_extension("backup");
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir)?;
        }
        println!("💾 备份原有索引: {:?} -> {:?}", index_dir, backup_dir);
        fs::rename(&index_dir, &backup_dir)?;
        index_backup = Some(backup_dir);
    } else {
        println!("ℹ️  未发现原有索引");
    }
    
    // 备份 embedding cache
    if cache_dir.exists() {
        let backup_dir = cache_dir.with_extension("backup");
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir)?;
        }
        println!("💾 备份原有 embedding cache: {:?} -> {:?}", cache_dir, backup_dir);
        fs::rename(&cache_dir, &backup_dir)?;
        cache_backup = Some(backup_dir);
    } else {
        println!("ℹ️  未发现原有 embedding cache");
    }
    
    if index_backup.is_some() || cache_backup.is_some() {
        println!("✓ 数据备份完成");
    }
    
    Ok((index_backup, cache_backup))
}

/// 恢复原有数据（索引 + embedding cache）
fn restore_data(index_backup: Option<PathBuf>, cache_backup: Option<PathBuf>) -> anyhow::Result<()> {
    let index_dir = get_index_dir()?;
    let cache_dir = get_embedding_cache_dir()?;
    
    // 删除测试产生的索引
    if index_dir.exists() {
        println!("🗑️  删除测试产生的索引: {:?}", index_dir);
        fs::remove_dir_all(&index_dir)?;
    }
    
    // 删除测试产生的 embedding cache
    if cache_dir.exists() {
        println!("🗑️  删除测试产生的 embedding cache: {:?}", cache_dir);
        fs::remove_dir_all(&cache_dir)?;
    }
    
    // 恢复索引
    if let Some(backup_path) = index_backup {
        println!("♻️  恢复原有索引: {:?} -> {:?}", backup_path, index_dir);
        fs::rename(&backup_path, &index_dir)?;
    } else {
        println!("ℹ️  无原有索引需要恢复");
    }
    
    // 恢复 embedding cache
    if let Some(backup_path) = cache_backup {
        println!("♻️  恢复原有 embedding cache: {:?} -> {:?}", backup_path, cache_dir);
        fs::rename(&backup_path, &cache_dir)?;
    } else {
        println!("ℹ️  无原有 embedding cache 需要恢复");
    }
    
    println!("✓ 数据恢复完成");
    Ok(())
}

/// 统计目录下的文件数量
fn count_files(dir: &Path) -> usize {
    if !dir.exists() {
        return 0;
    }
    
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                count += 1;
            } else if path.is_dir() {
                count += count_files(&path);
            }
        }
    }
    count
}

/// 执行索引命令并返回耗时（毫秒）
async fn run_index(index_path: &str) -> anyhow::Result<u64> {
    // 先统计文件数量
    let file_count = count_files(Path::new(index_path));
    println!("🔨 开始建立索引: {} (共 {} 个文件)", index_path, file_count);
    
    let pb = ProgressBar::new(file_count as u64);
    pb.set_style(ProgressStyle::with_template("{spinner} 索引中 [{elapsed}] | 文件 {pos}/{len} ({percent}%)")?
        .tick_strings(&["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"]));
    pb.enable_steady_tick(Duration::from_millis(120));

    let start_time = Instant::now();

    // 使用 spawn 而非 output，以便实时读取 stdout
    let mut child = Command::new("cargo")
        .args(&["run", "-p", "server", "--", "index", index_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    // 读取 stdout 来获取进度
    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut line = String::new();
    
    use tokio::io::AsyncBufReadExt;
    
    let mut total: u64 = file_count as u64;
    
    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }
        
        let trimmed = line.trim();
        
        // 解析进度信息
        if trimmed.starts_with("PROGRESS:TOTAL:") {
            if let Ok(t) = trimmed.strip_prefix("PROGRESS:TOTAL:").unwrap().parse::<u64>() {
                total = t;
                pb.set_length(total);
            }
        } else if trimmed.starts_with("PROGRESS:CURRENT:") {
            let parts: Vec<&str> = trimmed.strip_prefix("PROGRESS:CURRENT:").unwrap().split('/').collect();
            if parts.len() == 2 {
                if let Ok(current) = parts[0].parse::<u64>() {
                    pb.set_position(current);
                }
            }
        } else if trimmed == "PROGRESS:DONE" {
            pb.set_position(total);
        }
    }
    
    // 等待进程完成
    let status = child.wait().await?;
    let elapsed = start_time.elapsed().as_millis() as u64;

    if !status.success() {
        pb.abandon();
        return Err(anyhow::anyhow!("索引失败"));
    }

    pb.finish_and_clear();
    println!("✓ 索引完成 {} 个文件，耗时: {}ms ({:.2}s)", total, elapsed, elapsed as f64 / 1000.0);
    Ok(elapsed)
}

/// 启动 server 进程
async fn start_server() -> anyhow::Result<tokio::process::Child> {
    println!("🚀 启动 server 进程...");
    
    // 先确保 server 已编译
    println!("⏳ 编译 server...");
    let compile_start = Instant::now();
    let compile_status = Command::new("cargo")
        .args(&["build", "-p", "server"])
        .status()
        .await?;
    let compile_time = compile_start.elapsed();
    
    if !compile_status.success() {
        return Err(anyhow::anyhow!("编译 server 失败"));
    }
    println!("✓ Server 编译完成 ({:.1}s)", compile_time.as_secs_f64());
    
    // 使用编译好的二进制文件启动
    // 注意：继承 stderr 让我们能看到 server 的启动日志（包括 AI 模型加载进度）
    let child = Command::new("cargo")
        .args(&["run", "-p", "server", "--", "serve"])
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())  // 继承 stderr 以便看到 server 日志
        .spawn()?;
    
    // 等待 server 启动
    println!("⏳ 等待 server 启动（包括 AI 模型加载）...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    println!("✓ Server 已启动");
    Ok(child)
}

/// 等待 server 就绪（能够建立连接），超时则返回错误
async fn wait_for_server_ready(socket_path: &Path, timeout_secs: u64) -> anyhow::Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner} 等待 server 就绪... {elapsed}")?
        .tick_strings(&["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"]));
    pb.enable_steady_tick(Duration::from_millis(120));

    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let mut socket_found = false;
    let mut last_log = Instant::now();

    loop {
        // 先检查 socket 文件是否存在
        if socket_path.exists() {
            if !socket_found {
                pb.println(format!("ℹ️  Socket 文件已创建 ({:.1}s)，尝试连接...", start.elapsed().as_secs_f64()));
                socket_found = true;
            }
            
            // 尝试实际连接，确认 server 真的就绪
            let connect_start = Instant::now();
            match tokio::net::UnixStream::connect(socket_path).await {
                Ok(_stream) => {
                    let connect_time = connect_start.elapsed().as_millis();
                    pb.println(format!("✓ 连接成功 (耗时: {}ms)，等待 server 初始化...", connect_time));
                    
                    // 连接成功，但需要等待一小会让 server 完全就绪
                    drop(_stream);
                    let init_start = Instant::now();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    let _init_time = init_start.elapsed().as_millis();
                    
                    pb.finish_and_clear();
                    println!("✓ Server 就绪 (总耗时: {:.1}s)", start.elapsed().as_secs_f64());
                    return Ok(());
                }
                Err(e) => {
                    // socket 存在但连接失败，每 2 秒打印一次
                    if last_log.elapsed().as_millis() > 2000 {
                        pb.println(format!("⚠️  连接失败 ({:.1}s): {} - 继续等待...", start.elapsed().as_secs_f64(), e));
                        last_log = Instant::now();
                    }
                }
            }
        } else if socket_found {
            // Socket 文件消失了（server 崩溃？）
            pb.println("⚠️  Socket 文件已消失，server 可能崩溃了");
            socket_found = false;
        }

        if start.elapsed() >= timeout {
            pb.abandon();
            // 检查 server 进程是否还在
            let ps_output = std::process::Command::new("pgrep")
                .args(&["-f", "server.*serve"])
                .output();
            let server_running = ps_output.map(|o| o.status.success()).unwrap_or(false);
            
            return Err(anyhow::anyhow!(
                "等待 server 就绪超时 ({:.1}s): {:?}\nSocket 文件存在: {}\nServer 进程运行中: {}",
                start.elapsed().as_secs_f64(),
                socket_path,
                socket_path.exists(),
                server_running
            ));
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// 检查文件名中是否包含 title 的关键词
/// 检查 ZH 文件名是否匹配 title
fn check_title_match(file_path: &str, title: &str) -> bool {
    // 从文件路径中提取文件名（去掉编号前缀）
    let file_name = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    
    // 移除文件名开头的数字编号（如 001_）
    let file_name = file_name.trim_start_matches(|c: char| c.is_numeric() || c == '_');
    
    // 简单匹配：检查文件名是否包含 title 的主要部分
    // 移除 title 中的特殊字符进行比较
    let title_clean: String = title.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    
    let file_name_clean: String = file_name.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    
    // 检查是否包含主要关键词（取 title 前 20 个字符作为关键词）
    let key_words: String = title_clean.chars().take(20).collect();
    
    file_name_clean.contains(&key_words) || 
    key_words.chars().take(10).collect::<String>().len() > 0 && 
    file_name_clean.contains(&key_words.chars().take(10).collect::<String>())
}

/// 检查 EN 文件名是否在预期文件列表中
fn check_file_match(file_path: &str, expected_files: &[String]) -> bool {
    if expected_files.is_empty() {
        return false;
    }
    
    let file_name = Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    
    expected_files.iter().any(|expected| expected == file_name)
}

/// 执行单个测试用例
async fn run_test_case(
    client: &WorldClient,
    test_case: &TestCase,
    search_mode: SearchMode,
) -> anyhow::Result<TestResult> {
    let start_time = std::time::Instant::now();
    
    let req = SearchRequest {
        query: test_case.question.clone(),
        search_mode,
    };
    
    let session_id = match client.start_search(context::current(), req).await? {
        Ok(id) => id,
        Err(_e) => {
            let expected = if let Some(title) = &test_case.title {
                title.clone()
            } else {
                format!("One of: {}", test_case.expected_files.join(", "))
            };
            
            return Ok(TestResult {
                question: test_case.question.clone(),
                expected,
                found: false,
                rank: None,
                total_results: 0,
                search_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }
    };
    
    // 等待搜索完成
    let mut total_count: usize = 0;
    loop {
        tokio::time::sleep(Duration::from_millis(50)).await;

        let (_req_id, status_result) = client.search_status(context::current(), session_id).await?;
        match status_result {
            Ok(status) => match status {
                SearchStatus::Completed { total_count: count } => {
                    total_count = count as usize;
                    break;
                }
                SearchStatus::Failed(_) | SearchStatus::Cancelled => {
                    break;
                }
                SearchStatus::InProgress { .. } => {
                    // 继续等待
                }
            },
            Err(_) => break,
        }
    }

    // 获取前 20 个结果检查
    let fetch_req = FetchSearchResultsRequest {
        session_id,
        offset: 0,
        limit: 20,
    };

    let mut found = false;
    let mut rank = None;

    if let Ok((_req_id, Ok(results))) = client.fetch_search_results(context::current(), fetch_req).await {
        for (idx, hit) in results.hits.iter().enumerate() {
            let file_path_str = hit.file_path.to_string_lossy();
            
            // 根据是 ZH 还是 EN 选择不同的匹配方式
            let is_match = if let Some(title) = &test_case.title {
                // ZH: 检查文件名是否包含 title
                check_title_match(&file_path_str, title)
            } else {
                // EN: 检查文件名是否在预期文件列表中
                check_file_match(&file_path_str, &test_case.expected_files)
            };
            
            if is_match {
                found = true;
                rank = Some(idx + 1);
                break;
            }
        }
    }
    
    let search_time_ms = start_time.elapsed().as_millis() as u64;
    
    let expected = if let Some(title) = &test_case.title {
        title.clone()
    } else {
        format!("One of: {}", test_case.expected_files.join(", "))
    };
    
    Ok(TestResult {
        question: test_case.question.clone(),
        expected,
        found,
        rank,
        total_results: total_count,
        search_time_ms,
    })
}

/// 保存详细结果到 CSV
fn save_results_csv(results: &[TestResult], output_path: &Path) -> anyhow::Result<()> {
    let mut file = File::create(output_path)?;
    
    // 写入表头
    writeln!(file, "question,expected,found,rank,total_results,search_time_ms")?;
    
    // 写入每条结果
    for result in results {
        writeln!(
            file,
            "\"{}\",\"{}\",{},{},{},{}",
            result.question.replace("\"", "\"\""),
            result.expected.replace("\"", "\"\""),
            result.found,
            result.rank.map(|r| r.to_string()).unwrap_or_else(|| "N/A".to_string()),
            result.total_results,
            result.search_time_ms
        )?;
    }
    
    Ok(())
}

/// 生成测试报告
fn generate_report(
    results: &[TestResult],
    index_time_ms: u64,
    output_path: &Path,
) -> anyhow::Result<()> {
    let mut file = File::create(output_path)?;
    
    let total_tests = results.len();
    let found_count = results.iter().filter(|r| r.found).count();
    let accuracy = (found_count as f64 / total_tests as f64) * 100.0;
    
    let top1_count = results.iter().filter(|r| r.rank == Some(1)).count();
    let top3_count = results.iter().filter(|r| r.rank.map(|r| r <= 3).unwrap_or(false)).count();
    let top5_count = results.iter().filter(|r| r.rank.map(|r| r <= 5).unwrap_or(false)).count();
    let top10_count = results.iter().filter(|r| r.rank.map(|r| r <= 10).unwrap_or(false)).count();
    
    let avg_time: f64 = results.iter().map(|r| r.search_time_ms as f64).sum::<f64>() / total_tests as f64;
    let total_search_time: u64 = results.iter().map(|r| r.search_time_ms).sum();
    
    writeln!(file, "==========================================")?;
    writeln!(file, "       Benchmark 测试报告")?;
    writeln!(file, "==========================================")?;
    writeln!(file)?;
    writeln!(file, "测试时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file)?;
    writeln!(file, "【索引性能】")?;
    writeln!(file, "索引时间: {}ms ({:.2}s)", index_time_ms, index_time_ms as f64 / 1000.0)?;
    writeln!(file)?;
    writeln!(file, "【搜索准确率】")?;
    writeln!(file, "总测试数: {}", total_tests)?;
    writeln!(file, "成功找到: {} / {} ({:.2}%)", found_count, total_tests, accuracy)?;
    writeln!(file)?;
    writeln!(file, "【排名分布】")?;
    writeln!(file, "Top-1:  {} ({:.2}%)", top1_count, (top1_count as f64 / total_tests as f64) * 100.0)?;
    writeln!(file, "Top-3:  {} ({:.2}%)", top3_count, (top3_count as f64 / total_tests as f64) * 100.0)?;
    writeln!(file, "Top-5:  {} ({:.2}%)", top5_count, (top5_count as f64 / total_tests as f64) * 100.0)?;
    writeln!(file, "Top-10: {} ({:.2}%)", top10_count, (top10_count as f64 / total_tests as f64) * 100.0)?;
    writeln!(file)?;
    writeln!(file, "【搜索性能】")?;
    writeln!(file, "平均搜索时间: {:.2}ms", avg_time)?;
    writeln!(file, "总搜索时间: {}ms ({:.2}s)", total_search_time, total_search_time as f64 / 1000.0)?;
    writeln!(file)?;
    
    // 失败案例
    let failed_cases: Vec<_> = results.iter().filter(|r| !r.found).collect();
    if !failed_cases.is_empty() {
        writeln!(file, "【未找到的测试用例】({}个)", failed_cases.len())?;
        writeln!(file, "==========================================")?;
        for (idx, result) in failed_cases.iter().enumerate() {
            writeln!(file, "[{}] 问题: {}", idx + 1, result.question)?;
            writeln!(file, "    期望: {}", result.expected)?;
            writeln!(file)?;
        }
    }
    
    Ok(())
}

/// 解析命令行参数
fn parse_args() -> (Option<usize>, String) {
    let args: Vec<String> = std::env::args().collect();
    let mut limit = None;
    let mut lang = "ZH".to_string();  // 默认中文
    
    // 查找 --limit 参数
    for i in 0..args.len() {
        if args[i] == "--limit" || args[i] == "-l" {
            if i + 1 < args.len() {
                if let Ok(n) = args[i + 1].parse::<usize>() {
                    limit = Some(n);
                }
            }
        }
        // 查找 --lang 参数
        if args[i] == "--lang" {
            if i + 1 < args.len() {
                let l = args[i + 1].to_uppercase();
                if l == "ZH" || l == "EN" {
                    lang = l;
                }
            }
        }
    }
    
    (limit, lang)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 解析命令行参数
    let (limit, lang) = parse_args();
    
    // 构建数据集路径
    let benchmark_base = Path::new("benchmark");
    let lang_dir = benchmark_base.join(&lang);
    
    // 确定提取目录
    // ZH: benchmark/ZH/docs/extracted
    // EN: benchmark/EN/processed
    let extracted_dir = if lang == "EN" {
        lang_dir.join("processed")
    } else {
        lang_dir.join("docs").join("extracted")
    };
    
    let csv_path = lang_dir.join("card.csv");
    let result_csv_path = lang_dir.join("result.csv");
    let report_path = lang_dir.join("report.txt");
    
    if let Some(n) = limit {
        println!("🚀 开始 Benchmark 自动化测试 (语言: {}, Debug 模式: 测试前 {} 个文件)", lang, n);
    } else {
        println!("🚀 开始 Benchmark 自动化测试 (语言: {})", lang);
    }
    println!("{}", "=".repeat(60));
    
    // 检查必要的目录和文件
    if !extracted_dir.exists() {
        return Err(anyhow::anyhow!("提取目录不存在: {:?}", extracted_dir));
    }
    let keyword_index_path = lang_dir.join("keyword_index.json");
    if !keyword_index_path.exists() && !csv_path.exists() {
        return Err(anyhow::anyhow!("测试数据不存在 (需要 keyword_index.json 或 card.csv): {:?}", lang_dir));
    }
    
    // 步骤 0: 杀掉可能存在的旧 server 进程
    println!();
    kill_existing_server().await?;
    
    // 步骤 1: 备份原有数据
    println!();
    let (index_backup, cache_backup) = backup_data()?;
    
    println!();
    
    // 步骤 1.5: 准备测试数据（如果是 debug 模式，拷贝文件到临时目录）
    let (index_dir, temp_dir) = prepare_test_data(&extracted_dir, limit)?;
    let index_path = index_dir.to_string_lossy().to_string();
    let index_time_ms = run_index(&index_path).await?;
    
    println!();
    
    // 步骤 2: 启动 server
    let mut server_process = start_server().await?;
    
    println!();
    
    // 步骤 3: 连接到服务器
    let strategy = config::create_strategy()?;
    let runtime_dir = strategy.runtime_dir().unwrap_or_else(|| std::env::temp_dir().join("unnamed"));
    let socket_path = runtime_dir.join(config::constants::UNIX_SOCKET_FILE_NAME);
    
    println!("📡 连接到服务器: {:?}", socket_path);

    // 等待 server 真正就绪（能建立连接）
    // 注意：AI 模型加载可能需要较长时间，所以超时设为 180 秒
    if let Err(e) = wait_for_server_ready(&socket_path, 180).await {
        eprintln!("✗ 等待 server 就绪失败: {}", e);
        server_process.kill().await?;
        restore_data(index_backup, cache_backup)?;
        return Err(e);
    }
    
    let transport = tarpc::serde_transport::unix::connect(&socket_path, Bincode::default).await?;
    let client = WorldClient::new(client::Config::default(), transport).spawn();
    
    // 测试连接
    match client.ping(context::current()).await {
        Ok(response) => println!("✓ 服务器响应: {}", response),
        Err(e) => {
            eprintln!("✗ 无法连接到服务器: {}", e);
            server_process.kill().await?;
            restore_data(index_backup, cache_backup)?;
            return Err(e.into());
        }
    }
    
    // 加载测试用例（从 keyword_index.json）
    println!("\n📋 加载测试用例: {:?}", lang_dir);
    
    let mut test_cases = load_test_cases(&lang_dir, &lang)?;
    
    // 如果是 debug 模式，只保留前 N 个测试用例
    if let Some(n) = limit {
        test_cases.truncate(n);
        println!("✓ Debug 模式: 只测试前 {} 个用例", test_cases.len());
    } else {
        println!("✓ 共加载 {} 个测试用例", test_cases.len());
    }
    
    // 运行测试（使用 Natural 模式）
    println!("\n{}", "=".repeat(60));
    println!("🧪 开始测试（搜索模式: Natural - AI 语义搜索）");
    println!("{}", "=".repeat(60));
    
    let mut results = Vec::new();
    let total = test_cases.len();

    let pb = ProgressBar::new(total as u64);
    pb.set_style(ProgressStyle::with_template(
        "{spinner} [{elapsed_precise}] 问题 {pos}/{len} | {wide_msg}"
    )?.progress_chars("#>-"));
    pb.set_message("准备开始");
    
    for (idx, test_case) in test_cases.iter().enumerate() {
        pb.set_message(format!("[{}/{}] {}", idx + 1, total, test_case.question));
        
        let result = run_test_case(&client, test_case, SearchMode::Natural).await?;
        
        if result.found {
            pb.println(format!("[{}] ✓ 找到 (排名: {}, {}ms)", idx + 1, result.rank.unwrap(), result.search_time_ms));
        } else {
            pb.println(format!("[{}] ✗ 未找到 ({}ms)", idx + 1, result.search_time_ms));
        }
        
        results.push(result);

        pb.inc(1);
        
        // 避免过快请求
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    pb.finish_with_message("测试完成");
    
    // 统计结果
    println!("\n{}", "=".repeat(60));
    println!("📊 测试结果统计");
    println!("{}", "=".repeat(60));
    
    let total_tests = results.len();
    let found_count = results.iter().filter(|r| r.found).count();
    let accuracy = (found_count as f64 / total_tests as f64) * 100.0;
    
    let top1_count = results.iter().filter(|r| r.rank == Some(1)).count();
    let top3_count = results.iter().filter(|r| r.rank.map(|r| r <= 3).unwrap_or(false)).count();
    let top5_count = results.iter().filter(|r| r.rank.map(|r| r <= 5).unwrap_or(false)).count();
    let top10_count = results.iter().filter(|r| r.rank.map(|r| r <= 10).unwrap_or(false)).count();
    
    let avg_time: f64 = results.iter().map(|r| r.search_time_ms as f64).sum::<f64>() / total_tests as f64;
    
    println!("\n总测试数: {}", total_tests);
    println!("成功找到: {} / {} ({:.2}%)", found_count, total_tests, accuracy);
    println!("\n排名分布:");
    println!("  Top-1:  {} ({:.2}%)", top1_count, (top1_count as f64 / total_tests as f64) * 100.0);
    println!("  Top-3:  {} ({:.2}%)", top3_count, (top3_count as f64 / total_tests as f64) * 100.0);
    println!("  Top-5:  {} ({:.2}%)", top5_count, (top5_count as f64 / total_tests as f64) * 100.0);
    println!("  Top-10: {} ({:.2}%)", top10_count, (top10_count as f64 / total_tests as f64) * 100.0);
    println!("\n平均搜索时间: {:.2}ms", avg_time);
    
    // 显示失败的案例
    let failed_cases: Vec<_> = results.iter().filter(|r| !r.found).collect();
    if !failed_cases.is_empty() {
        println!("\n{}", "=".repeat(60));
        println!("❌ 未找到的测试用例 ({}个):", failed_cases.len());
        println!("{}", "=".repeat(60));
        
        for (idx, result) in failed_cases.iter().enumerate() {
            println!("[{}] 问题: {}", idx + 1, result.question);
            println!("    期望: {}", result.expected);
        }
    }
    
    // 保存结果到 CSV
    println!("\n💾 保存详细结果到: {:?}", result_csv_path);
    save_results_csv(&results, &result_csv_path)?;
    println!("✓ result.csv 已保存");
    
    // 生成报告
    println!("💾 生成测试报告到: {:?}", report_path);
    generate_report(&results, index_time_ms, &report_path)?;
    println!("✓ report.txt 已生成");
    
    println!("\n✅ Benchmark 测试完成！");
    
    // 清理：关闭 server
    println!("\n🛑 关闭 server 进程...");
    server_process.kill().await?;
    
    // 等待 server 完全关闭
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // 清理临时测试目录
    if temp_dir.is_some() {
        println!();
        cleanup_test_data(temp_dir)?;
    }
    
    // 恢复原有数据
    println!();
    restore_data(index_backup, cache_backup)?;
    
    Ok(())
}
