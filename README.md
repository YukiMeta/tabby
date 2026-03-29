# Tabby 🐱

**你的终端 AI 工作台** - 在同一个终端里管理多个 AI 对话项目，每个项目独立上下文，切换=切换记忆。

```
╭──────────────────────────────────────╮
│  🐱 Tabby  ● 项目 A │ ○ 项目 B      │
├──────────────────────────────────────┤
│  [项目 A]                            │
│  > 帮我写个快速排序                   │
│  ◰ 好的...                           │
│                                      │
├──────────────────────────────────────┤
│ > _                                  │
├──────────────────────────────────────┤
│  Ctrl+q：退出  │  Tab：切换  │  📊   │
╰──────────────────────────────────────╯
```

## 核心特性

### 📑 项目隔离
每个项目独立上下文，切换 tab = 切换完整的对话记忆。

### 🧠 命令分流
| 输入 | 处理 |
|------|------|
| `!ls` | Shell 命令，直接执行 |
| `/help` | 内部命令，Tabby 功能 |
| `你好` | 发送给 AI |

### 📊 监测面板
`Ctrl+m` 查看所有项目的今日进度。

## 快速开始

```bash
# 1. 克隆
git clone https://github.com/yourusername/tabby.git
cd tabby

# 2. 编译
cargo build --release

# 3. 运行
./target/release/tabby
```

或者安装到系统路径：
```bash
./install.sh
tabby  # 直接运行
```

## 快捷键

| 按键 | 功能 |
|------|------|
| `Ctrl+q` | 退出 |
| `Tab` / `Shift+Tab` | 切换项目 |
| `Ctrl+n` | 新建项目 |
| `Ctrl+m` | 监测面板 |
| `Alt+1/2/3` | 跳转到项目 1/2/3 |
| `Enter` | 发送 |
| `?` | 帮助 |

## 命令

```
!ls -la        # Shell 命令
/new 项目名      # 新建项目
/list          # 项目列表
/clear         # 清空对话
```

## 配置

配置文件位于 `~/.config/tabby/config.toml`

```toml
claude_api_key = "sk-..."
claude_base_url = "https://..."
openai_api_key = "sk-..."
```

或使用环境变量：
```bash
export ANTHROPIC_API_KEY="sk-..."
```

## 技术栈

- Rust
- Ratatui (TUI)
- Crossterm (终端)
- Serde (配置)
- Chrono (时间)

## License

MIT

---

🐱 Tabby - 你的终端 AI 工作台
