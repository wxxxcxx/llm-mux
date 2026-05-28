use std::path::PathBuf;

use clap::{Parser, Subcommand};

use llm_mux_gateway::config::Config;
use llm_mux_gateway::{init_tracing, server};

#[derive(Parser)]
#[command(name = "llm-mux", about = "LLM API 协议互转网关")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动网关服务
    Start {
        /// 监听端口
        #[arg(long, default_value = "8080")]
        port: u16,
        /// 监听地址
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// 配置文件路径
        #[arg(long, default_value = "config.yaml")]
        config: PathBuf,
        /// 日志级别
        #[arg(long, default_value = "info")]
        log_level: String,
        /// 以守护进程方式运行
        #[arg(long)]
        daemon: bool,
        /// PID 文件路径
        #[arg(long)]
        pid_file: Option<PathBuf>,
    },
    /// 停止运行中的网关
    Stop {
        /// PID 文件路径
        #[arg(long)]
        pid_file: Option<PathBuf>,
        /// 等待超时时间（秒）
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
    /// 管理配置文件
    #[command(subcommand)]
    Config(ConfigCmd),
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// 生成默认配置文件
    Init {
        /// 输出路径
        #[arg(long, default_value = "config.yaml")]
        path: PathBuf,
        /// 强制覆盖已存在的文件
        #[arg(long)]
        force: bool,
    },
    /// 校验配置文件
    Validate {
        /// 配置文件路径
        #[arg(long, default_value = "config.yaml")]
        path: PathBuf,
    },
    /// 展示当前配置
    Show {
        /// 配置文件路径
        #[arg(long, default_value = "config.yaml")]
        path: PathBuf,
    },
}

fn daemonize() {
    #[cfg(unix)]
    {
        match unsafe { libc::fork() } {
            -1 => {
                eprintln!("failed to fork");
                std::process::exit(1);
            }
            0 => unsafe {
                libc::setsid();
            },
            _ => {
                std::process::exit(0);
            }
        }
    }
    #[cfg(not(unix))]
    {
        eprintln!("--daemon is not supported on Windows; running in foreground");
    }
}

fn write_pid_file(path: &std::path::Path) {
    let pid = std::process::id().to_string();
    if let Err(e) = std::fs::write(path, &pid) {
        eprintln!(
            "warning: failed to write PID file {}: {}",
            path.display(),
            e
        );
    }
}

fn read_pid_file(path: &std::path::Path) -> Option<u32> {
    let content = std::fs::read_to_string(path).ok()?;
    content.trim().parse().ok()
}

fn remove_pid_file(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
}

fn is_process_running(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}

fn kill_process(pid: u32, force: bool) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = std::process::Command::new("taskkill");
        cmd.arg("/PID").arg(pid.to_string());
        if force {
            cmd.arg("/F");
        }
        cmd.creation_flags(CREATE_NO_WINDOW)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(unix)]
    {
        let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
        unsafe { libc::kill(pid as i32, signal) == 0 }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Config(cmd) => match cmd {
            ConfigCmd::Init { path, force } => {
                if path.exists() && !force {
                    eprintln!(
                        "error: {} already exists. Use --force to overwrite.",
                        path.display()
                    );
                    std::process::exit(1);
                }
                let default = Config::generate_default();
                if let Err(e) = std::fs::write(&path, &default) {
                    eprintln!("error: failed to write {}: {}", path.display(), e);
                    std::process::exit(1);
                }
                println!("config written to {}", path.display());
            }
            ConfigCmd::Validate { path } => match Config::from_file(&path) {
                Ok(cfg) => {
                    if let Err(e) = cfg.validate() {
                        eprintln!("validation failed: {}", e);
                        std::process::exit(1);
                    }
                    println!("config is valid");
                }
                Err(e) => {
                    eprintln!("error: failed to load config: {}", e);
                    std::process::exit(1);
                }
            },
            ConfigCmd::Show { path } => match Config::from_file(&path) {
                Ok(cfg) => {
                    println!("{}", cfg.display());
                }
                Err(e) => {
                    eprintln!("error: failed to load config: {}", e);
                    std::process::exit(1);
                }
            },
        },
        Commands::Stop { pid_file, timeout } => {
            let pid_path = pid_file.unwrap_or_else(|| PathBuf::from("llm-mux.pid"));
            let pid = match read_pid_file(&pid_path) {
                Some(p) => p,
                None => {
                    eprintln!("error: no PID file found at {}", pid_path.display());
                    std::process::exit(1);
                }
            };
            println!("stopping process {} ...", pid);

            if !is_process_running(pid) {
                eprintln!("process {} not found or already stopped", pid);
                remove_pid_file(&pid_path);
                std::process::exit(0);
            }

            kill_process(pid, false);

            let start = std::time::Instant::now();
            loop {
                if start.elapsed().as_secs() > timeout {
                    println!("timeout reached, sending force kill");
                    kill_process(pid, true);
                    break;
                }
                if !is_process_running(pid) {
                    println!("process {} stopped", pid);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            remove_pid_file(&pid_path);
        }
        Commands::Start {
            port,
            host,
            config,
            log_level,
            daemon,
            pid_file,
        } => {
            if daemon {
                daemonize();
            }

            if let Some(ref pid_path) = pid_file {
                write_pid_file(pid_path);
            }

            init_tracing(&log_level);

            let cfg = match Config::from_file(&config) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("failed to load config: {}", e);
                    std::process::exit(1);
                }
            };

            if let Err(e) = cfg.validate() {
                tracing::error!("config validation failed: {}", e);
                std::process::exit(1);
            }

            let cfg = {
                let mut c = cfg;
                c.host = host;
                c.port = port;
                c.log_level = log_level;
                c
            };

            let srv = server::Server::new(cfg).unwrap_or_else(|e| {
                tracing::error!("failed to create server: {}", e);
                std::process::exit(1);
            });

            if let Err(e) = srv.serve().await {
                tracing::error!("server error: {}", e);
                std::process::exit(1);
            }

            if let Some(ref pid_path) = pid_file {
                remove_pid_file(pid_path);
            }
        }
    }
}
