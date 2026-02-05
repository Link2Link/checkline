use std::path::Path;
use std::process;

use clap::{Parser, ValueEnum};

/// 树形目录结构显示工具
#[derive(Parser, Debug)]
#[command(name = "checkline")]
#[command(author = "checkline")]
#[command(version = "1.0.0")]
#[command(about = "树形目录结构显示工具", long_about = None)]
struct Args {
    /// 目录路径，默认为当前目录
    #[arg(default_value = ".", value_name = "DIR")]
    directory: Option<String>,

    /// 最大显示深度
    #[arg(short, long, value_name = "N")]
    depth: Option<usize>,

    /// 显示隐藏文件（以.开头的文件）
    #[arg(short, long)]
    all: bool,

    /// 显示文件大小
    #[arg(short, long)]
    size: bool,

    /// 只显示源码文件
    #[arg(short, long)]
    code: bool,

    /// 显示源码文件行数
    #[arg(short = 'l', long)]
    lines: bool,

    /// 排除注释行，只统计有效代码行数
    #[arg(long)]
    no_comments: bool,

    /// 排序方式（name, time, size）
    #[arg(short, long, value_enum, default_value = "name")]
    sort: SortOrder,

    /// 排除指定文件或文件夹（可多次使用）
    #[arg(short, long)]
    exclude: Vec<String>,

    /// 使用 glob 模式排除（可多次使用），如 "*.lock", "target/**"
    #[arg(short = 'g', long)]
    glob: Vec<String>,
}

/// 排序方式
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum SortOrder {
    /// 按名称排序
    #[value(name = "name")]
    Name,
    /// 按修改时间排序
    #[value(name = "time")]
    Time,
    /// 按文件大小排序
    #[value(name = "size")]
    Size,
}

/// 命令行解析后用于传递各项配置信息的结构体
struct Config {
    dir: String,
    max_depth: usize,
    show_hidden: bool,
    show_size: bool,
    code_only: bool,
    show_lines: bool,
    no_comments: bool,
    exclude: Vec<String>,
    glob: Vec<String>,
    sort: SortOrder,
}

/// 支持的源码文件扩展名列表
const CODE_EXTENSIONS: &[&str] = &[
    // Rust
    "rs",
    // Python
    "py", "pyx",
    // C/C++
    "c", "h", "cpp", "cc", "cxx", "hpp", "hxx", "h",
    // CMake
    "cmake", "cmake.in",
];

/// 用于统计各类代码文件行数
#[derive(Default)]
struct LineStats {
    rust: u64,
    python: u64,
    c_cpp: u64,
    cmake: u64,
    total: u64,
}

impl LineStats {
    /// 按类型累加行数
    fn add(&mut self, category: &str, lines: u64) {
        self.total += lines;
        match category {
            "rust" => self.rust += lines,
            "python" => self.python += lines,
            "c_cpp" => self.c_cpp += lines,
            "cmake" => self.cmake += lines,
            _ => {}
        }
    }

    /// 格式化输出每种类型的行数
    fn format(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.rust > 0 {
            parts.push(format!("Rust: {}", self.rust));
        }
        if self.python > 0 {
            parts.push(format!("Python: {}", self.python));
        }
        if self.c_cpp > 0 {
            parts.push(format!("C/C++: {}", self.c_cpp));
        }
        if self.cmake > 0 {
            parts.push(format!("CMake: {}", self.cmake));
        }
        parts.join(", ")
    }
}

/// 返回文件所属类别（rust、python、c_cpp、cmake），否则为None
fn get_file_category(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_string_lossy();

    // CMakeLists.txt 特殊处理
    if name == "CMakeLists.txt" {
        return Some("cmake");
    }

    // 扩展名匹配
    if let Some(ext) = path.extension() {
        match ext.to_string_lossy().to_lowercase().as_str() {
            "rs" => Some("rust"),
            "py" | "pyx" => Some("python"),
            "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some("c_cpp"),
            "cmake" | "cmake.in" => Some("cmake"),
            _ => None,
        }
    } else {
        None
    }
}

/// 判断是否是支持的源码文件
fn is_source_code(file_name: &str) -> bool {
    // CMakeLists.txt 特殊处理
    if file_name == "CMakeLists.txt" {
        return true;
    }

    // 检查扩展名是否在支持列表里
    if let Some(ext) = Path::new(file_name).extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        CODE_EXTENSIONS.contains(&ext_str.as_str())
    } else {
        false
    }
}

/// 检查文件名是否在排除列表中
fn is_excluded(file_name: &str, exclude_list: &[String]) -> bool {
    exclude_list.iter().any(|pattern| file_name == pattern)
}

/// 简单的通配符匹配，支持 * 和 **
fn matches_wildcard(name: &str, pattern: &str) -> bool {
    // 递归实现通配符匹配
    fn helper(name: &[u8], pattern: &[u8]) -> bool {
        if pattern.is_empty() {
            return name.is_empty();
        }
        if name.is_empty() {
            return pattern.iter().all(|&c| c == b'*');
        }

        match pattern[0] {
            b'*' => {
                // * 可以匹配零个或多个字符
                helper(&name, &pattern[1..]) || // * 匹配零个
                (!name.is_empty() && helper(&name[1..], pattern)) // * 匹配一个，回递归
            }
            b'?' => {
                // ? 匹配任意单个字符
                helper(&name[1..], &pattern[1..])
            }
            c => {
                // 普通字符必须匹配
                name[0] == c && helper(&name[1..], &pattern[1..])
            }
        }
    }

    helper(name.as_bytes(), pattern.as_bytes())
}

/// 检查文件名是否匹配 glob 模式
fn matches_glob(file_name: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| matches_wildcard(file_name, pattern))
}

/// 检查文件是否应该被排除（精确匹配或 glob 模式）
fn should_exclude(file_name: &str, exclude: &[String], glob_patterns: &[String]) -> bool {
    is_excluded(file_name, exclude) || matches_glob(file_name, glob_patterns)
}

/// 统计文件总行数，不区分注释
fn count_lines(path: &Path) -> std::io::Result<u64> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.lines().map(|_| 1).sum())
}

/// 统计文件有效代码行（排除注释）
fn count_code_lines(path: &Path) -> std::io::Result<u64> {
    let content = std::fs::read_to_string(path)?;
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());

    // Python: # 开头为注释行
    if ext == Some("py".to_string()) {
        let mut count = 0;
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('#') && !trimmed.is_empty() {
                count += 1;
            }
        }
        return Ok(count);
    }

    // C/C++/Rust: 支持 // 和 /* */ 注释的过滤
    let mut in_multiline_comment = false;
    let mut count = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        if in_multiline_comment {
            // 在多行注释内，若本行包含 */ 结束多行注释
            if let Some(end) = trimmed.find("*/") {
                in_multiline_comment = false;
                let after = &trimmed[end + 2..].trim();
                // 多行注释后的内容若非空且不以 // 开头，记为有效行
                if !after.is_empty() && !after.starts_with("//") {
                    count += 1;
                }
            }
            // 否则：仍处于多行注释，跳过
        } else {
            if trimmed.starts_with("/*") {
                // 行内包含多行注释，若同一行有 */, 检查后续内容
                if let Some(end) = trimmed.find("*/") {
                    let after = &trimmed[end + 2..].trim();
                    if !after.is_empty() && !after.starts_with("//") {
                        count += 1;
                    }
                } else {
                    in_multiline_comment = true;
                }
            } else if trimmed.starts_with("//") {
                // 单行注释，跳过
            } else if !trimmed.is_empty() {
                // 普通非空有效行
                count += 1;
            }
        }
    }

    Ok(count)
}

/// 递归统计目录下源代码行数
fn count_dir_lines(path: &Path, max_depth: usize, current_depth: usize, no_comments: bool, exclude: &[String], glob_patterns: &[String]) -> std::io::Result<LineStats> {
    if current_depth > max_depth {
        return Ok(LineStats::default());
    }

    let mut stats = LineStats::default();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // 排除隐藏文件和指定排除的项
        if name_str.starts_with('.') || should_exclude(&name_str, exclude, glob_patterns) {
            continue;
        }

        if entry_path.is_dir() {
            // 是目录则递归统计
            let child_stats = count_dir_lines(&entry_path, max_depth, current_depth + 1, no_comments, exclude, glob_patterns)?;
            stats.rust += child_stats.rust;
            stats.python += child_stats.python;
            stats.c_cpp += child_stats.c_cpp;
            stats.cmake += child_stats.cmake;
            stats.total += child_stats.total;
        } else if is_source_code(&name_str) {
            // 是源码文件则累加统计行数
            if let Some(category) = get_file_category(&entry_path) {
                let lines = if no_comments {
                    count_code_lines(&entry_path)?
                } else {
                    count_lines(&entry_path)?
                };
                stats.add(category, lines);
            }
        }
    }
    Ok(stats)
}

/// 文件大小格式化成友好的字符串
fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if size >= GB {
        format!("{:.2}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2}K", size as f64 / KB as f64)
    } else {
        format!("{}B", size)
    }
}

/// 以树形结构递归打印文件夹，并显示额外信息
fn print_tree(
    dir: &Path,
    prefix: &str,
    depth: usize,
    config: &Config,
) -> std::io::Result<()> {
    // 超过最大深度直接返回
    if depth > config.max_depth {
        return Ok(());
    }

    // 读取目录下所有文件和文件夹，应用过滤条件
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        // 过滤隐藏文件
        .filter(|e| {
            let name = e.file_name();
            if config.show_hidden {
                true
            } else {
                !name.to_string_lossy().starts_with('.')
            }
        })
        // 只显示源码文件时，过滤非源码文件
        .filter(|e| {
            let path = e.path();
            if path.is_dir() {
                true
            } else if config.code_only {
                is_source_code(&e.file_name().to_string_lossy())
            } else {
                true
            }
        })
        // 排除指定文件或文件夹（精确匹配或 glob 模式）
        .filter(|e| {
            let binding = e.file_name();
            let name = binding.to_string_lossy();
            !should_exclude(&name, &config.exclude, &config.glob)
        })
        .collect();

    // 排序
    match config.sort {
        SortOrder::Name => entries.sort_by_key(|e| e.path()),
        SortOrder::Time => entries.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        }),
        SortOrder::Size => entries.sort_by_key(|e| {
            e.metadata()
                .map(|m| m.len())
                .unwrap_or(0)
        }),
    }

    let total: usize = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        let is_last_entry = i == total - 1;
        let file_name = entry.file_name();
        let path = entry.path();
        let is_dir = path.is_dir();

        // 树形结构字符
        let connector = if is_last_entry { "└── " } else { "├── " };
        let new_prefix = if is_last_entry { "    " } else { "│   " };

        let mut output = format!("{}{}", prefix, connector);

        if is_dir {
            // 目录蓝色显示
            output.push_str("\x1b[1;34m"); // 蓝色目录
            output.push_str(&file_name.to_string_lossy());
            output.push_str("\x1b[0m");
            // 显示目录下代码行数
            if config.show_lines {
                if let Ok(stats) = count_dir_lines(&path, config.max_depth, depth + 1, config.no_comments, &config.exclude, &config.glob) {
                    output.push_str(&format!(" {:>6}", stats.total));
                }
            }
        } else {
            // 普通文件
            output.push_str(&file_name.to_string_lossy());
            // 显示文件大小
            if config.show_size {
                if let Ok(metadata) = entry.metadata() {
                    let size = metadata.len();
                    output.push_str(&format!(" {}", format_file_size(size)));
                }
            }
            // 显示文件行数
            if config.show_lines && is_source_code(&file_name.to_string_lossy()) {
                let lines = if config.no_comments {
                    count_code_lines(&path)?
                } else {
                    count_lines(&path)?
                };
                output.push_str(&format!(" {:>6}", lines));
            }
        }

        // 打印输出本行
        println!("{}", output);

        // 若是目录，递归处理其子项
        if is_dir {
            let new_prefix = format!("{}{}", prefix, new_prefix);
            print_tree(&path, &new_prefix, depth + 1, config)?;
        }
    }

    Ok(())
}

/// 主函数，解析参数、初始化配置并启动主流程
fn main() {
    // 解析命令行参数
    let args = Args::parse();

    // 汇集参数为配置
    let config = Config {
        dir: args.directory.unwrap_or_else(|| ".".to_string()),
        max_depth: args.depth.unwrap_or(usize::MAX),
        show_hidden: args.all,
        show_size: args.size,
        code_only: args.code,
        show_lines: args.lines,
        no_comments: args.no_comments,
        exclude: args.exclude,
        glob: args.glob,
        sort: args.sort,
    };

    // 获取目录路径
    let path = Path::new(&config.dir);

    // 检查目录是否存在
    if !path.exists() {
        eprintln!("错误: 目录 '{}' 不存在", config.dir);
        process::exit(1);
    }

    // 检查路径是否为目录
    if !path.is_dir() {
        eprintln!("错误: '{}' 不是一个目录", config.dir);
        process::exit(1);
    }

    // 打印根目录名
    println!("{}", config.dir);

    // 打印树形目录结构
    if let Err(e) = print_tree(path, "", 0, &config) {
        eprintln!("错误: {}", e);
        process::exit(1);
    }

    // 若显示代码行数，额外输出总行数统计
    if config.show_lines {
        if let Ok(stats) = count_dir_lines(path, usize::MAX, 0, config.no_comments, &config.exclude, &config.glob) {
            println!("\n总计: {} 行", stats.total);
            if !stats.format().is_empty() {
                println!("  {}", stats.format());
            }
        }
    }
}
