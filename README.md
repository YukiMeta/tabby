# Yuki ❄️

日式极简风格的终端 AI 标签页工具。在同一个终端窗口中切换使用多个 AI 服务，支持智能命令分流。

## 效果预览

```
╭──────────────────────────────────────╮
│  ❄️ Yuki  一 Claude │ 二 OpenCode   │
├──────────────────────────────────────┤
│  [Claude]                            │
│  ◰ 你好，有什么可以帮你？             │
│  > 请帮我写一个快速排序               │
│                                      │
│  思考中...                           │
├──────────────────────────────────────┤
│ > 输入内容 █                         │
├──────────────────────────────────────┤
│  Ctrl+q：退出  │  Tab：切换  │  API: ✓│
╰──────────────────────────────────────╯
```

## 核心特性

### 🧠 智能命令分流

| 输入前缀 | 处理方式 | 示例 |
|---------|---------|------|
| `!` | Shell 命令 - 直接在终端执行 | `!ls -la` `!cargo build` |
| `/` | 内部命令 - Yuki 内置功能 | `/help` `/clear` |
| 其他 | AI 消息 - 发送到当前标签的 AI | `你好` `写个排序` |

### 📑 多标签页管理
- 一 Claude - Anthropic Claude API
- 二 OpenCode - OpenAI API
- 三 Codex - 自定义 API
- 支持动态添加更多标签

## 安装

### 从源码安装

```bash
# 1. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 克隆并编译
git clone <repo-url>
cd yuki
cargo build --release

# 3. 安装
./install.sh
```

### 快速安装

```bash
cd yuki
./install.sh
```

## 使用

```bash
yuki
```

### 命令示例

```
# Shell 命令
!ls -la          # 查看文件
!cargo build     # 编译项目
!gh auth login   # GitHub 登录
!git status      # Git 状态

# 内部命令
/help            # 显示帮助
/clear           # 清空对话
/config          # 查看配置
/reload          # 重载配置
/add <标签名>     # 添加新标签

# AI 对话（直接输入）
你好             # 发送到 AI
请帮我写个快速排序  # 发送到 AI
```

## 快捷键

| 按键 | 功能 |
|------|------|
| `Ctrl+q` | 退出应用 |
| `Tab` | 切换到下一个标签 |
| `Shift+Tab` | 切换到上一个标签 |
| `Alt+1/2/3` | 跳转到指定标签 |
| `Enter` | 发送消息 |
| `?` | 显示/隐藏帮助 |

## 配置

配置文件位于 `~/.config/yuki/config.toml`

```toml
# Claude API
claude_api_key = "sk-..."
claude_base_url = "https://api.anthropic.com"

# OpenAI API
openai_api_key = "sk-..."

# 自定义 API
custom_api_key = ""
custom_base_url = ""
```

或使用环境变量：
```bash
export ANTHROPIC_API_KEY="sk-..."
export OPENAI_API_KEY="sk-..."
```

## 标签页说明

| 标签 | 用途 | API |
|------|------|-----|
| 一 Claude | Anthropic Claude | Claude API |
| 二 OpenCode | OpenAI 服务 | OpenAI API |
| 三 Codex | 自定义服务 | Custom API |

## 技术栈

- Rust
- Ratatui (TUI 框架)
- Crossterm (终端操作)
- Tokio (异步运行时)
- Reqwest (HTTP 客户端)

## Roadmap

- [x] 基础 TUI 框架
- [x] 标签页切换
- [x] 配置文件支持
- [x] 命令分流系统（Shell/内部/AI）
- [ ] 真实 API 集成
- [ ] 流式响应
- [ ] 对话历史保存
- [ ] 自定义标签页
- [ ] 主题配置
- [ ] Markdown 渲染

## License

MIT

---

❄️ Yuki - 极简 AI 终端
