#!/bin/bash
# Zen-Tabs 安装脚本

set -e

echo "🏯 禅・タブ - 安装程序"
echo ""

# 检测架构
ARCH=$(uname -m)
OS=$(uname -s)

echo "检测到系统：$OS ($ARCH)"

# 创建目录
INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/zen-tabs"

mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR"

# 复制二进制文件
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cp "$SCRIPT_DIR/target/release/zen-tabs" "$INSTALL_DIR/"

# 设置权限
chmod +x "$INSTALL_DIR/zen-tabs"

# 创建默认配置
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    cat > "$CONFIG_DIR/config.toml" << 'EOF'
# 禅・タブ 配置文件

# Claude API 配置
claude_api_key = ""
claude_base_url = ""

# OpenAI API 配置
openai_api_key = ""

# 自定义 API 配置
custom_api_key = ""
custom_base_url = ""
EOF
    echo "✓ 创建默认配置文件：$CONFIG_DIR/config.toml"
fi

# 添加到 PATH 提示
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo ""
    echo "⚠️  需要将 ~/.local/bin 添加到 PATH"
    echo ""
    echo "请添加到 ~/.zshrc 或 ~/.bashrc:"
    echo '  export PATH="$HOME/.local/bin:$PATH"'
    echo ""
fi

echo ""
echo "✅ 安装完成！"
echo ""
echo "运行方式："
echo "  zen-tabs"
echo ""
echo "或："
echo "  $INSTALL_DIR/zen-tabs"
echo ""
