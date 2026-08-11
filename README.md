# 🐯Tabby 

**你的终端 AI 工作台** - 在同一个终端里管理多个 AI 对话项目，每个项目独立上下文，切换=切换记忆。

```
╭──────────────────────────────────────╮
│  🐯 Tabby  ● 项目 A │ ○ 项目 B      │
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

---

## 🚀 快速开始（3 分钟上手）

### 步骤 1：检查环境

```bash
# 需要 Rust 环境，没有的话一键安装
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 验证安装
rustc --version    # 应该显示 rustc 1.x.x
cargo --version    # 应该显示 cargo 1.x.x
```

### 步骤 2：下载 Tabby

```bash
git clone https://github.com/YukiMeta/tabby.git
cd tabby
```

### 步骤 3：编译

```bash
# 编译 release 版本（约 1-2 分钟）
cargo build --release
```

### 步骤 4：运行

```bash
# 方式 A：直接运行
./target/release/tabby

# 方式 B：安装到系统路径（推荐）
./install.sh
tabby  # 之后可以直接运行
```

### 步骤 5：配置 API（可选）

Tabby 需要配置 AI API 才能对话。配置文件位于 `~/.config/tabby/config.toml`：

```toml
claude_api_key = "sk-你的密钥"
claude_base_url = "https://你的代理地址"
```

或使用环境变量：
```bash
export ANTHROPIC_API_KEY="sk-..."
export ANTHROPIC_BASE_URL="https://..."
```

---

## 📖 使用指南

### 核心概念

Tabby 的每个项目都有**独立的对话历史**，就像浏览器的多个标签页：

| 场景 | 用法 |
|------|------|
| 同时开发多个项目 | 每个项目一个 Tabby 标签 |
| 切换工作上下文 | `Tab` 键切换，记忆完整保留 |
| 查看进度 | `Ctrl+m` 打开监测面板 |

### 命令分流

Tabby 根据输入前缀决定如何处理：

| 前缀 | 示例 | 处理方 |
|------|------|--------|
| `!` | `!ls -la` | Shell 命令，直接执行 |
| `/` | `/new 项目名` | Tabby 内部命令 |
| 无 | `写个排序` | 发送给 AI |

### 快捷键

| 按键 | 功能 |
|------|------|
| `Ctrl+q` | 退出 |
| `Tab` / `Shift+Tab` | 切换项目 |
| `Ctrl+n` | 新建项目 |
| `Ctrl+m` | 监测面板 |
| `Alt+1/2/3` | 跳转到项目 1/2/3 |
| `Enter` | 发送 |

---

## 🛠️ 常见问题

### Q: 运行时提示 "Device not configured"

**A**: Tabby 是 TUI 应用，必须在真正的 macOS 终端.App 中运行，不能在 IDE 输出面板或聊天工具内运行。

### Q: 编译失败

**A**: 确保 Rust 版本 >= 1.70：
```bash
rustup update
cargo clean
cargo build --release
```

### Q: 配置不生效

**A**: 检查配置文件位置：
```bash
cat ~/.config/tabby/config.toml
```

### Q: 数据存在哪里？

**A**: 项目数据保存在 `~/.config/tabby/projects/<项目名>/project.json`

---

## 📦 技术栈

- **Rust** - 系统编程语言
- **Ratatui 0.29** - TUI 渲染库
- **Crossterm 0.28** - 终端操作
- **Serde + TOML** - 配置序列化
- **Chrono** - 时间处理

---

## 📄 License

MIT

---

**🐱 Tabby - 你的终端 AI 工作台**

遇到问题？在 GitHub 提 Issue：https://github.com/YukiMeta/tabby/issues
