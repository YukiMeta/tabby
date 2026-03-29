#!/bin/bash
# Tabby 🐱 安装脚本

set -e

echo "🐱 Tabby - 安装程序"
echo ""

# 创建目录
INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/tabby"

mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR"

# 复制二进制文件
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cp "$SCRIPT_DIR/target/release/tabby" "$INSTALL_DIR/"

# 设置权限
chmod +x "$INSTALL_DIR/tabby"

# 创建默认配置
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    cat > "$CONFIG_DIR/config.toml" << 'EOF'
# Tabby 🐱 配置文件

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

echo ""
echo "✅ 安装完成！"
echo ""
echo "运行方式："
echo "  tabby"
echo ""
