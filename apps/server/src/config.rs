use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use config::{create_strategy, resolve_dir, AppStrategy};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", default="default_config", deny_unknown_fields)]
pub struct Config {
    pub runtime_dir: PathBuf,
    pub cache_dir: PathBuf,
    /// 要监控和索引的目录列表
    pub watch_paths: Vec<PathBuf>,
}


fn default_config() -> Config {
    let strategy = create_strategy().unwrap();

    Config {
        runtime_dir: resolve_dir("RUNTIME_DIRECTORY", &strategy, |s| {
            s.runtime_dir()
        }),
        cache_dir: resolve_dir("CACHE_DIRECTORY", &strategy, |s| {
            Some(s.cache_dir())
        }),
        watch_paths: vec![],  // 默认为空，要求用户配置
    }
}
    

impl Config {
    fn load_str(user_config_str: &str) -> Result<Config> {
        let user_config: Config = toml::from_str(user_config_str)?;
        Ok(user_config)
    }

    pub fn load() -> Result<Config> {
        let strategy = create_strategy()?;
        let config_path = strategy.config_dir().join(config::constants::SERVER_CONFIG_FILE_NAME);

        match std::fs::read_to_string(&config_path) {
            Ok(user_config_str) => Self::load_str(&user_config_str),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // 配置文件不存在，创建示例配置文件
                Self::create_example_config(&config_path)?;
                Self::load_str("")
            }
            Err(e) => Err(e.into()),
        }
    }

    fn create_example_config(config_path: &PathBuf) -> Result<()> {
        use std::io::Write;
        
        // 确保配置目录存在
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let example_config = r#"# Server 配置文件
#
# 此文件在首次运行时自动创建
# 配置修改后重启服务生效

# 要监控和索引的目录列表
# 建议配置你经常需要搜索的目录
watch-paths = [
    # "/Users/yourname/Documents",
    # "/Users/yourname/Projects",
]

# 可选：自定义运行时目录
# runtime-dir = "/custom/runtime/path"

# 可选：自定义缓存目录
# cache-dir = "/custom/cache/path"
"#;

        let mut file = std::fs::File::create(config_path)?;
        file.write_all(example_config.as_bytes())?;
        
        eprintln!("\n📝 已创建配置文件: {:?}", config_path);
        eprintln!("💡 请编辑配置文件，添加要索引的目录到 watch-paths");
        eprintln!("   然后运行: cargo run -p server -- index\n");
        
        Ok(())
    }
}
