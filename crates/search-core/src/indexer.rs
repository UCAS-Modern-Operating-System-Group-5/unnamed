// search-core/src/indexer.rs
//! 索引模块 - 文件索引和监控

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::SystemTime;
use anyhow::Result;
use std::sync::Arc;

use ignore::WalkBuilder;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use tantivy::schema::*;
use tantivy::{Index, doc, IndexWriter, Term, IndexReader, ReloadPolicy};
use tantivy_jieba::JiebaTokenizer;

use crate::ai::BertModel;
use crate::cache::{EmbeddingCache, FileStatus};
use crate::config::CONFIG;
use crate::extract::extract_text;
use crate::registry::{FileRegistry, EventType};
use crate::schema::{build_schema, FIELD_TITLE, FIELD_BODY, FIELD_PATH, FIELD_TAGS, FIELD_FILE_SIZE, FIELD_MODIFIED_TIME, FIELD_CREATED_TIME, FIELD_ACCESSED_TIME};

/// 初始化持久化索引
pub fn init_persistent_index(index_path: &Path) -> Result<(Index, Schema, IndexReader)> {
    // 使用统一的 schema 构建器
    let schema = build_schema();

    if !index_path.exists() {
        fs::create_dir_all(index_path)?;
    }

    let index = Index::open_or_create(
        tantivy::directory::MmapDirectory::open(index_path)?, 
        schema.clone()
    )?;

    let tokenizer = JiebaTokenizer {};
    index.tokenizers().register("jieba", tokenizer);

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()?;

    Ok((index, schema, reader))
}

/// 从索引中删除文件
pub fn delete_from_index(
    file_path: &Path, 
    index: &Index, 
    schema: &Schema, 
    cache: Option<&EmbeddingCache>
) -> Result<bool> {
    let path_str = file_path.canonicalize()
        .unwrap_or_else(|_| file_path.to_path_buf())
        .to_string_lossy()
        .to_string();
    
    let path_field = schema.get_field(FIELD_PATH).unwrap();
    let mut index_writer: IndexWriter = index.writer(50_000_000)?;
    
    let path_term = Term::from_field_text(path_field, &path_str);
    index_writer.delete_term(path_term);
    
    let original_path_str = file_path.to_string_lossy();
    if original_path_str != path_str {
        let original_term = Term::from_field_text(path_field, &original_path_str);
        index_writer.delete_term(original_term);
    }
    
    index_writer.commit()?;
    
    if let Some(c) = cache {
        let _ = c.remove(&path_str);
        let _ = c.remove(&original_path_str);
        let _ = c.remove_file_meta(&path_str);
        let _ = c.remove_file_meta(&original_path_str);
    }
    
    tracing::info!("已从索引删除: {}", path_str);
    Ok(true)
}

/// 处理并索引单个文件
pub fn process_and_index(
    file_path: &Path, 
    index: &Index, 
    schema: &Schema, 
    bert: &BertModel, 
    cache: &EmbeddingCache
) -> Result<()> {
    let doc_data = extract_text(file_path)?;

    let file_timestamp = fs::metadata(file_path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::now())
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let file_created = fs::metadata(file_path)
        .and_then(|m| m.created())
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let file_accessed = fs::metadata(file_path)
        .and_then(|m| m.accessed())
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let file_size = fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);

    // AI 关键词提取（优先使用缓存）
    let keywords = if let Some(cached_keywords) = cache.get_keywords(&doc_data.path, &doc_data.content) {
        tracing::debug!("缓存命中: {:?}", cached_keywords);
        cached_keywords
    } else {
        tracing::debug!("正在分析文档语义...");
        let new_keywords = bert.extract_keywords(&doc_data.content, 3)?;
        let _ = cache.set_keywords(&doc_data.path, &doc_data.content, new_keywords.clone());
        tracing::debug!("生成标签: {:?}", new_keywords);
        new_keywords
    };
    let tags_str = keywords.join(" ");

    let title_field = schema.get_field(FIELD_TITLE).unwrap();
    let body_field = schema.get_field(FIELD_BODY).unwrap();
    let path_field = schema.get_field(FIELD_PATH).unwrap();
    let tags_field = schema.get_field(FIELD_TAGS).unwrap();
    let modified_time_field = schema.get_field(FIELD_MODIFIED_TIME).unwrap();
    let created_time_field = schema.get_field(FIELD_CREATED_TIME).unwrap();
    let accessed_time_field = schema.get_field(FIELD_ACCESSED_TIME).unwrap();
    let size_field = schema.get_field(FIELD_FILE_SIZE).unwrap();
    
    let mut index_writer: IndexWriter = index.writer(50_000_000)?;

    // 先删除旧文档
    let path_term = Term::from_field_text(path_field, &doc_data.path);
    index_writer.delete_term(path_term);

    // 写入新文档
    index_writer.add_document(doc!(
        title_field => doc_data.title.as_str(),
        body_field => doc_data.content.as_str(),
        path_field => doc_data.path.as_str(),
        tags_field => tags_str,
        modified_time_field => file_timestamp,
        created_time_field => file_created,
        accessed_time_field => file_accessed,
        size_field => file_size
    ))?;

    index_writer.commit()?;
    
    // 保存元数据
    let _ = cache.save_file_meta(&doc_data.path, file_path);

    tracing::info!("已索引: {}", doc_data.title);
    Ok(())
}

/// 清理孤儿索引
pub fn cleanup_orphan_indexes(index: &Index, schema: &Schema, cache: &EmbeddingCache) -> Result<usize> {
    let reader = index.reader()?;
    let searcher = reader.searcher();
    let path_field = schema.get_field("path").unwrap();
    
    let mut orphan_paths: Vec<String> = Vec::new();
    
    for segment_reader in searcher.segment_readers() {
        let store_reader = segment_reader.get_store_reader(1)?;
        for doc_id in 0..segment_reader.num_docs() {
            if let Ok(doc) = store_reader.get::<tantivy::TantivyDocument>(doc_id) {
                if let Some(path_value) = doc.get_first(path_field) {
                    if let Some(path_str) = path_value.as_str() {
                        let path = Path::new(path_str);
                        if !path.exists() {
                            tracing::info!("发现孤儿索引: {}", path_str);
                            orphan_paths.push(path_str.to_string());
                        }
                    }
                }
            }
        }
    }
    
    let orphan_count = orphan_paths.len();
    
    if orphan_count > 0 {
        let mut index_writer: IndexWriter = index.writer(50_000_000)?;
        for path_str in &orphan_paths {
            let path_term = Term::from_field_text(path_field, path_str);
            index_writer.delete_term(path_term);
            let _ = cache.remove(path_str);
            let _ = cache.remove_file_meta(path_str);
        }
        index_writer.commit()?;
        tracing::info!("已清理 {} 个孤儿索引", orphan_count);
    }
    
    // 清理元数据缓存中的孤儿
    let cached_paths = cache.get_all_cached_paths();
    let mut meta_orphan_count = 0;
    for path_str in cached_paths {
        let path = Path::new(&path_str);
        if !path.exists() {
            let _ = cache.remove(&path_str);
            let _ = cache.remove_file_meta(&path_str);
            meta_orphan_count += 1;
        }
    }
    if meta_orphan_count > 0 {
        tracing::info!("已清理 {} 个孤儿元数据缓存", meta_orphan_count);
    }
    
    Ok(orphan_count + meta_orphan_count)
}

/// 扫描现有文件
pub fn scan_existing_files(
    watch_path: &Path, 
    index: &Index, 
    schema: &Schema, 
    bert: &BertModel, 
    cache: &EmbeddingCache,
    registry: &FileRegistry,
) -> Result<()> {
    scan_existing_files_with_progress(watch_path, index, schema, bert, cache, registry, |_, _| {})
}

/// 扫描现有文件（带进度回调）
pub fn scan_existing_files_with_progress<F>(
    watch_path: &Path, 
    index: &Index, 
    schema: &Schema, 
    bert: &BertModel, 
    cache: &EmbeddingCache,
    registry: &FileRegistry,
    progress_callback: F,
) -> Result<()> 
where
    F: Fn(usize, usize) + Send + Sync,
{
    let _ = cleanup_orphan_indexes(index, schema, cache);
    
    // 先统计文件总数
    let total_files = count_supported_files(watch_path);
    tracing::info!("正在扫描现有文件... (共 {} 个支持的文件)", total_files);
    
    let mut file_count = 0;

    if CONFIG.walker.use_ripgrep_walker {
        scan_with_ripgrep_walker_progress(watch_path, index, schema, bert, cache, registry, &mut file_count, total_files, &progress_callback)?;
    } else {
        scan_with_std_walker_progress(watch_path, index, schema, bert, cache, registry, &mut file_count, total_files, &progress_callback)?;
    }
    
    tracing::info!("初始索引完成，共处理 {} 个文件", file_count);
    Ok(())
}

/// 统计目录下支持的文件数量
fn count_supported_files(dir: &Path) -> usize {
    if !dir.exists() {
        return 0;
    }
    
    let mut count = 0;
    
    if CONFIG.walker.use_ripgrep_walker {
        let walker_config = &CONFIG.walker;
        let mut builder = ignore::WalkBuilder::new(dir);
        builder
            .hidden(!walker_config.skip_hidden)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .ignore(walker_config.respect_ignore)
            .follow_links(walker_config.follow_symlinks);
        
        if walker_config.max_depth > 0 {
            builder.max_depth(Some(walker_config.max_depth));
        }
        
        for result in builder.build() {
            if let Ok(entry) = result {
                let path = entry.path();
                if !path.is_dir() && is_supported_file(path) {
                    count += 1;
                }
            }
        }
    } else {
        fn count_recursive(dir: &Path, count: &mut usize) {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && is_supported_file(&path) {
                        *count += 1;
                    } else if path.is_dir() {
                        count_recursive(&path, count);
                    }
                }
            }
        }
        count_recursive(dir, &mut count);
    }
    
    count
}

fn scan_with_ripgrep_walker_progress<F>(
    watch_path: &Path,
    index: &Index,
    schema: &Schema,
    bert: &BertModel,
    cache: &EmbeddingCache,
    registry: &FileRegistry,
    file_count: &mut usize,
    total_files: usize,
    progress_callback: &F,
) -> Result<()>
where
    F: Fn(usize, usize),
{
    let walker_config = &CONFIG.walker;
    
    let mut builder = ignore::WalkBuilder::new(watch_path);
    builder
        .hidden(!walker_config.skip_hidden)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .ignore(walker_config.respect_ignore)
        .follow_links(walker_config.follow_symlinks);
    
    if walker_config.max_depth > 0 {
        builder.max_depth(Some(walker_config.max_depth));
    }
    
    tracing::debug!("开始遍历目录: {:?}", watch_path);
    
    for result in builder.build() {
        match result {
            Ok(entry) => {
                let path = entry.path();
                tracing::debug!("发现条目: {:?}, is_dir={}, is_supported={}", 
                    path, path.is_dir(), is_supported_file(path));
                if path.is_dir() || !is_supported_file(path) {
                    continue;
                }
                process_file_entry(path, index, schema, bert, cache, registry, file_count);
                progress_callback(*file_count, total_files);
            }
            Err(e) => {
                tracing::warn!("遍历错误: {}", e);
            }
        }
    }
    
    Ok(())
}

fn scan_with_std_walker_progress<F>(
    watch_path: &Path,
    index: &Index,
    schema: &Schema,
    bert: &BertModel,
    cache: &EmbeddingCache,
    registry: &FileRegistry,
    file_count: &mut usize,
    total_files: usize,
    progress_callback: &F,
) -> Result<()>
where
    F: Fn(usize, usize),
{
    fn visit_dirs<F2>(
        dir: &Path, 
        index: &Index, 
        schema: &Schema, 
        file_count: &mut usize, 
        bert: &BertModel, 
        cache: &EmbeddingCache,
        registry: &FileRegistry,
        total_files: usize,
        progress_callback: &F2,
    ) -> Result<()>
    where
        F2: Fn(usize, usize),
    {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, index, schema, file_count, bert, cache, registry, total_files, progress_callback)?;
                } else if path.is_file() && is_supported_file(&path) {
                    process_file_entry(&path, index, schema, bert, cache, registry, file_count);
                    progress_callback(*file_count, total_files);
                }
            }
        }
        Ok(())
    }

    visit_dirs(watch_path, index, schema, file_count, bert, cache, registry, total_files, progress_callback)
}

fn scan_with_ripgrep_walker(
    watch_path: &Path,
    index: &Index,
    schema: &Schema,
    bert: &BertModel,
    cache: &EmbeddingCache,
    registry: &FileRegistry,
    file_count: &mut usize,
) -> Result<()> {
    let walker_config = &CONFIG.walker;
    
    let mut builder = WalkBuilder::new(watch_path);
    builder
        .hidden(!walker_config.skip_hidden)
        // 注意：用户明确指定要索引的目录，不应该被 .gitignore 排除
        // 所以禁用 gitignore，但保留其他 ignore 规则
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .ignore(walker_config.respect_ignore)
        .follow_links(walker_config.follow_symlinks);
    
    if walker_config.max_depth > 0 {
        builder.max_depth(Some(walker_config.max_depth));
    }
    
    tracing::debug!("开始遍历目录: {:?}", watch_path);
    
    for result in builder.build() {
        match result {
            Ok(entry) => {
                let path = entry.path();
                tracing::debug!("发现条目: {:?}, is_dir={}, is_supported={}", 
                    path, path.is_dir(), is_supported_file(path));
                if path.is_dir() || !is_supported_file(path) {
                    continue;
                }
                process_file_entry(path, index, schema, bert, cache, registry, file_count);
            }
            Err(e) => {
                tracing::warn!("遍历错误: {}", e);
            }
        }
    }
    
    Ok(())
}

fn scan_with_std_walker(
    watch_path: &Path,
    index: &Index,
    schema: &Schema,
    bert: &BertModel,
    cache: &EmbeddingCache,
    registry: &FileRegistry,
    file_count: &mut usize,
) -> Result<()> {
    fn visit_dirs(
        dir: &Path, 
        index: &Index, 
        schema: &Schema, 
        file_count: &mut usize, 
        bert: &BertModel, 
        cache: &EmbeddingCache,
        registry: &FileRegistry,
    ) -> Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, index, schema, file_count, bert, cache, registry)?;
                } else if path.is_file() && is_supported_file(&path) {
                    process_file_entry(&path, index, schema, bert, cache, registry, file_count);
                }
            }
        }
        Ok(())
    }

    visit_dirs(watch_path, index, schema, file_count, bert, cache, registry)
}

fn process_file_entry(
    path: &Path,
    index: &Index,
    schema: &Schema,
    bert: &BertModel,
    cache: &EmbeddingCache,
    registry: &FileRegistry,
    file_count: &mut usize,
) {
    let path_buf = path.to_path_buf();
    let path_str = path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string();
    
    let status = cache.check_file_status(&path_str, path);
    tracing::debug!("文件状态检查: {:?} -> {:?}", path.file_name().unwrap_or_default(), status);
    
    match status {
        FileStatus::Unchanged => return,
        FileStatus::New => {
            tracing::debug!("[新增] {}", path.file_name().unwrap_or_default().to_string_lossy());
        }
        FileStatus::Modified => {
            tracing::debug!("[变更] {}", path.file_name().unwrap_or_default().to_string_lossy());
        }
    }
    
    if let Some(modified_time) = get_modified_time(path) {
        if registry.try_start_processing(&path_buf, modified_time) {
            match process_and_index(path, index, schema, bert, cache) {
                Ok(_) => *file_count += 1,
                Err(e) => tracing::error!("处理文件失败 {:?}: {}", path, e),
            }
            registry.finish_processing(&path_buf);
        }
    }
}

fn is_supported_file(path: &Path) -> bool {
    if path.to_string_lossy().contains(".DS_Store") {
        return false;
    }
    
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        CONFIG.walker.supported_extensions
            .iter()
            .any(|supported| supported.eq_ignore_ascii_case(&ext))
    } else {
        false
    }
}

/// 检查文件扩展名是否支持（公开版本）
pub fn is_file_supported(path: &Path) -> bool {
    is_supported_file(path)
}

fn get_modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

/// 启动文件监控
pub fn start_file_watcher(
    watch_path: PathBuf, 
    index: Index, 
    schema: Schema, 
    bert: Arc<BertModel>, 
    cache: Arc<EmbeddingCache>,
    registry: FileRegistry,
) -> Sender<()> {
    let (scan_complete_tx, scan_complete_rx): (Sender<()>, Receiver<()>) = channel();
    
    thread::spawn(move || {
        let (tx, rx) = channel();
        let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("监控启动失败: {:?}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(&watch_path, RecursiveMode::Recursive) {
            tracing::error!("监控启动失败: {:?}", e);
            return;
        }

        tracing::info!("文件监控已启动: {:?}", watch_path);

        // 等待扫描完成，期间收集事件到 pending_events
        loop {
            // 非阻塞检查扫描是否完成
            match scan_complete_rx.try_recv() {
                Ok(()) => {
                    tracing::info!("扫描完成，开始处理实时事件");
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // 扫描未完成，收集事件到待处理队列
                    match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                        Ok(res) => {
                            if let Ok(event) = res {
                                let event_type = match event.kind {
                                    EventKind::Create(_) => Some(EventType::Create),
                                    EventKind::Modify(notify::event::ModifyKind::Data(_)) => Some(EventType::Modify),
                                    EventKind::Remove(_) => Some(EventType::Delete),
                                    _ => None,
                                };
                                if let Some(et) = event_type {
                                    for path in event.paths {
                                        if is_supported_file(&path) {
                                            registry.add_pending_event(path, et.clone());
                                        }
                                    }
                                }
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
            }
        }

        // 处理扫描期间的待处理事件（去重：只处理扫描后修改的文件）
        let pending_events = registry.complete_scan();
        for event in pending_events {
            if is_supported_file(&event.path) {
                // 检查文件是否在扫描时已经处理过且未再修改
                if let Some(file_mod_time) = get_modified_time(&event.path) {
                    if registry.is_file_processed(&event.path, file_mod_time) {
                        tracing::debug!("跳过已处理的文件: {:?}", event.path);
                        continue;
                    }
                }
                
                match event.event_type {
                    EventType::Create | EventType::Modify => {
                        let _ = process_and_index(&event.path, &index, &schema, &bert, &cache);
                    }
                    EventType::Delete => {
                        let _ = delete_from_index(&event.path, &index, &schema, Some(&cache));
                        registry.mark_deleted(&event.path);
                    }
                }
            }
        }

        // 处理实时事件
        for res in rx {
            match res {
                Ok(event) => {
                    tracing::debug!("收到文件事件: {:?}", event);
                    
                    let event_type = match event.kind {
                        EventKind::Create(_) => Some(EventType::Create),
                        EventKind::Modify(notify::event::ModifyKind::Data(_)) => Some(EventType::Modify),
                        EventKind::Remove(_) => Some(EventType::Delete),
                        _ => None,
                    };

                    let event_type = match event_type {
                        Some(t) => t,
                        None => continue,
                    };

                    for path in event.paths {
                        if !is_supported_file(&path) {
                            continue;
                        }
                        
                        // 使用 registry 防止重复处理
                        let path_buf = path.to_path_buf();
                        if let Some(modified_time) = get_modified_time(&path) {
                            if !registry.try_start_processing(&path_buf, modified_time) {
                                tracing::debug!("跳过正在处理或已处理的文件: {:?}", path);
                                continue;
                            }
                        }

                        match event_type {
                            EventType::Create | EventType::Modify => {
                                if !path.exists() {
                                    let _ = delete_from_index(&path, &index, &schema, Some(&cache));
                                    registry.mark_deleted(&path_buf);
                                } else {
                                    let _ = process_and_index(&path, &index, &schema, &bert, &cache);
                                }
                            }
                            EventType::Delete => {
                                let _ = delete_from_index(&path, &index, &schema, Some(&cache));
                                registry.mark_deleted(&path_buf);
                            }
                        }
                        
                        registry.finish_processing(&path_buf);
                    }
                }
                Err(e) => tracing::error!("Watch error: {:?}", e),
            }
        }
    });

    scan_complete_tx
}
