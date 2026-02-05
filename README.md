# checkline

一个用 Rust 编写的命令行工具，用于以树形结构显示目录内容，并统计源码行数。

## 功能特性

- 树形展示目录结构
- 支持多种编程语言源码统计
- 排除注释行统计有效代码
- 支持 glob 模式排除文件/文件夹
- 按语言分类统计
- 支持排序和深度限制

## 安装

### 从源码编译

```bash
cargo build --release
```

编译后的二进制文件位于 `target/release/checkline`

### 安装到系统

```bash
cargo install --path .
```

## 使用方法

```bash
checkline [OPTIONS] [DIR]
```

### 命令行选项

| 选项 | 说明 |
|------|------|
| `DIR` | 目录路径（默认当前目录） |
| `-d, --depth <N>` | 最大显示深度 |
| `-a, --all` | 显示隐藏文件 |
| `-s, --size` | 显示文件大小 |
| `-c, --code` | 只显示源码文件 |
| `-l, --lines` | 显示源码文件行数 |
| `--no-comments` | 排除注释行 |
| `-e, --exclude <NAME>` | 排除指定文件/文件夹 |
| `-g, --glob <PATTERN>` | 使用 glob 模式排除 |
| `-t, --sort <MODE>` | 排序方式：`name`, `time`, `size` |
| `-h, --help` | 显示帮助 |
| `-V, --version` | 显示版本 |

## 示例

### 基本使用

```bash
# 显示当前目录结构
checkline

# 显示指定目录
checkline /path/to/project

# 限制深度为2层
checkline -d 2
```

### 源码统计

```bash
# 只显示源码文件
checkline -c

# 显示源码行数
checkline -c -l

# 排除注释行
checkline -c -l --no-comments
```

### 排除文件

```bash
# 排除指定文件夹
checkline -c -l -e target -e .git

# 使用 glob 模式排除
checkline -c -l -g "*.lock" -g "*.log"

# 组合使用
checkline -c -l -e target -g "*.pyc"
```

### 完整统计

```bash
checkline -c -l --no-comments /project
```

输出示例：

```
/project
├── src
│   └── main.rs    250
├── lib
│   └── utils.rs    120
└── CMakeLists.txt     10

总计: 380 行
  Rust: 370, CMake: 10
```

## 支持的语言

| 语言 | 扩展名 |
|------|--------|
| Rust | `.rs` |
| Python | `.py`, `.pyx` |
| C/C++ | `.c`, `.h`, `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx` |
| CMake | `CMakeLists.txt`, `.cmake` |

## 注释排除规则

### Python
- `#` 开头的行视为注释

### C/C++, Rust
- `//` 开头的行视为注释
- `/* ... */` 多行注释

## License

本项目采用 [Apache License 2.0](LICENSE) 开源协议。详见 [LICENSE](LICENSE) 文件。
