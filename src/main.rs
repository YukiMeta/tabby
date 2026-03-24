use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, stdout},
    process::Command,
    time::Duration,
};

// ==================== 配置结构 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ApiConfig {
    claude_api_key: Option<String>,
    claude_base_url: Option<String>,
    openai_api_key: Option<String>,
    custom_api_key: Option<String>,
    custom_base_url: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            claude_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            claude_base_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            custom_api_key: std::env::var("CUSTOM_API_KEY").ok(),
            custom_base_url: std::env::var("CUSTOM_BASE_URL").ok(),
        }
    }
}

fn load_or_create_config() -> ApiConfig {
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join("yuki")
        .join("config.toml");

    if config_path.exists() {
        let content = fs::read_to_string(&config_path).unwrap_or_default();
        ApiConfig {
            claude_api_key: extract_value(&content, "claude_api_key"),
            claude_base_url: extract_value(&content, "claude_base_url"),
            openai_api_key: extract_value(&content, "openai_api_key"),
            custom_api_key: extract_value(&content, "custom_api_key"),
            custom_base_url: extract_value(&content, "custom_base_url"),
        }
    } else {
        let config = ApiConfig::default();
        let _ = fs::create_dir_all(config_path.parent().unwrap());
        let _ = fs::write(&config_path, format!(
            "# Yuki ❄️ 配置文件\n\n\
             claude_api_key = \"{}\"\n\
             claude_base_url = \"{}\"\n\
             openai_api_key = \"\"\n\
             custom_api_key = \"\"\n\
             custom_base_url = \"\"\n",
            config.claude_api_key.clone().unwrap_or_default(),
            config.claude_base_url.clone().unwrap_or_default()
        ));
        config
    }
}

fn extract_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        if line.starts_with(key) {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let value = parts[1].trim().trim_matches('"');
                return if value.is_empty() { None } else { Some(value.to_string()) };
            }
        }
    }
    None
}

// ==================== 消息结构 ====================

#[derive(Clone, Debug)]
enum MessageRole {
    User,
    Assistant,
    System,
    ShellOutput,
}

#[derive(Clone, Debug)]
struct Message {
    role: MessageRole,
    content: String,
}

// ==================== Tab 数据结构 ====================

#[derive(Clone, Debug)]
struct Tab {
    name: String,
    messages: Vec<Message>,
    input: String,
    cursor_pos: usize,
    is_loading: bool,
    api_type: ApiType,
}

#[derive(Clone, Debug)]
enum ApiType {
    Claude,
    OpenAI,
    Custom,
}

impl Tab {
    fn new(name: &str, api_type: ApiType) -> Self {
        Self {
            name: name.to_string(),
            messages: vec![Message {
                role: MessageRole::System,
                content: format!("欢迎使用 {}", name),
            }],
            input: String::new(),
            cursor_pos: 0,
            is_loading: false,
            api_type,
        }
    }

    fn api_key(&self, config: &ApiConfig) -> Option<String> {
        match self.api_type {
            ApiType::Claude => config.claude_api_key.clone(),
            ApiType::OpenAI => config.openai_api_key.clone(),
            ApiType::Custom => config.custom_api_key.clone(),
        }
    }
}

// ==================== 应用状态 ====================

struct App {
    tabs: Vec<Tab>,
    active_tab: usize,
    should_quit: bool,
    show_help: bool,
    config: ApiConfig,
    status_message: String,
}

impl App {
    fn new() -> Self {
        let config = load_or_create_config();

        Self {
            tabs: vec![
                Tab::new("Claude", ApiType::Claude),
                Tab::new("OpenCode", ApiType::OpenAI),
                Tab::new("Codex", ApiType::Custom),
            ],
            active_tab: 0,
            should_quit: false,
            show_help: false,
            config,
            status_message: String::from("准备就绪"),
            last_command_output: None,
        }
    }

    fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
    }

    fn previous_tab(&mut self) {
        self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
    }

    /// 执行 shell 命令
    fn run_shell_command(&mut self, cmd: &str) {
        self.status_message = format!("执行：!{}", cmd);

        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output();

        match output {
            Ok(out) => {
                let result = if !out.stdout.is_empty() {
                    String::from_utf8_lossy(&out.stdout).to_string()
                } else if !out.stderr.is_empty() {
                    String::from_utf8_lossy(&out.stderr).to_string()
                } else {
                    "命令执行成功（无输出）".to_string()
                };

                let tab = self.active_tab_mut();
                tab.messages.push(Message {
                    role: MessageRole::ShellOutput,
                    content: format!("$ {}\n{}", cmd, result),
                });
                tab.is_loading = false;
                self.status_message = "命令执行完成".to_string();
            }
            Err(e) => {
                let tab = self.active_tab_mut();
                tab.messages.push(Message {
                    role: MessageRole::ShellOutput,
                    content: format!("$ {}\n错误：{}", cmd, e),
                });
                self.status_message = "命令执行失败".to_string();
            }
        }
    }

    /// 执行内部命令（/ 开头）
    fn run_internal_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let command = parts.get(0).unwrap_or(&"");

        match *command {
            "help" | "?" => {
                self.show_help = true;
            }
            "clear" => {
                let tab = self.active_tab_mut();
                tab.messages.clear();
                tab.messages.push(Message {
                    role: MessageRole::System,
                    content: "对话已清空".to_string(),
                });
                self.status_message = "对话已清空".to_string();
            }
            "config" => {
                let claude_status = if self.config.claude_api_key.is_some() { "已配置" } else { "未配置" };
                let openai_status = if self.config.openai_api_key.is_some() { "已配置" } else { "未配置" };
                let config_info = format!(
                    "配置文件：~/.config/yuki/config.toml\n\
                     Claude API: {}\n\
                     OpenAI API: {}",
                    claude_status,
                    openai_status
                );
                let tab = self.active_tab_mut();
                tab.messages.push(Message {
                    role: MessageRole::System,
                    content: config_info,
                });
                self.status_message = "配置信息已显示".to_string();
            }
            "reload" => {
                self.config = load_or_create_config();
                self.status_message = "配置已重新加载".to_string();
            }
            "tab" | "add" => {
                if parts.len() > 1 {
                    let new_tab_name = parts[1];
                    self.tabs.push(Tab::new(new_tab_name, ApiType::Custom));
                    self.status_message = format!("已添加标签：{}", new_tab_name);
                } else {
                    self.status_message = "用法：/add <标签名>".to_string();
                }
            }
            _ => {
                self.status_message = format!("未知命令：/{}", command);
            }
        }
    }

    /// 发送消息到 AI
    fn send_to_ai(&mut self, input: &str) {
        let tab = self.active_tab_mut();

        tab.messages.push(Message {
            role: MessageRole::User,
            content: input.to_string(),
        });
        tab.is_loading = true;
        tab.input.clear();
        tab.cursor_pos = 0;

        self.status_message = format!("正在发送请求到 {}...", self.active_tab().name);

        // 模拟 API 响应
        let response = simulate_api_response(input, &self.active_tab().name);

        {
            let tab = self.active_tab_mut();
            tab.messages.push(Message {
                role: MessageRole::Assistant,
                content: response,
            });
            tab.is_loading = false;
        }
        self.status_message = "回复已完成".to_string();
    }

    fn process_input(&mut self) {
        let input = {
            let tab = self.active_tab();
            tab.input.trim().to_string()
        };

        if input.is_empty() {
            return;
        }

        // 命令分流逻辑
        if input.starts_with('!') {
            // Shell 命令
            let cmd = input.trim_start_matches('!').trim();
            self.run_shell_command(cmd);
        } else if input.starts_with('/') {
            // 内部命令
            let cmd = input.trim_start_matches('/').trim();
            self.run_internal_command(cmd);
        } else {
            // 发送给 AI
            self.send_to_ai(&input);
        }

        // 清空输入框
        let tab = self.active_tab_mut();
        tab.input.clear();
        tab.cursor_pos = 0;
    }

    fn handle_input(&mut self, key: event::KeyEvent) {
        match key.code {
            event::KeyCode::Char('q') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            event::KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            event::KeyCode::Tab => {
                if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                    self.previous_tab();
                } else {
                    self.next_tab();
                }
            }
            event::KeyCode::Char('1') if key.modifiers.contains(event::KeyModifiers::ALT) => {
                self.active_tab = 0;
            }
            event::KeyCode::Char('2') if key.modifiers.contains(event::KeyModifiers::ALT) => {
                self.active_tab = 1;
            }
            event::KeyCode::Char('3') if key.modifiers.contains(event::KeyModifiers::ALT) => {
                self.active_tab = 2;
            }
            event::KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            event::KeyCode::Char('r') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.status_message = "已刷新".to_string();
            }
            event::KeyCode::Char(c) => {
                let tab = self.active_tab_mut();
                tab.input.insert(tab.cursor_pos, c);
                tab.cursor_pos += 1;
            }
            event::KeyCode::Backspace => {
                let tab = self.active_tab_mut();
                if tab.cursor_pos > 0 {
                    tab.cursor_pos -= 1;
                    tab.input.remove(tab.cursor_pos);
                }
            }
            event::KeyCode::Enter => {
                self.process_input();
            }
            event::KeyCode::Left => {
                let tab = self.active_tab_mut();
                if tab.cursor_pos > 0 {
                    tab.cursor_pos -= 1;
                }
            }
            event::KeyCode::Right => {
                let tab = self.active_tab_mut();
                if tab.cursor_pos < tab.input.len() {
                    tab.cursor_pos += 1;
                }
            }
            _ => {}
        }
    }
}

fn simulate_api_response(input: &str, service: &str) -> String {
    format!("[{}] 收到：{}\n\n这是一条模拟回复。配置真实 API 后即可使用。", service, input)
}

// ==================== 渲染函数 ====================

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    render_tabs(f, app, chunks[0]);
    render_content(f, app, chunks[1]);
    render_input(f, app, chunks[2]);
    render_status_bar(f, app, chunks[3]);

    if app.show_help {
        render_help_popup(f, f.area());
    }
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let style = if i == app.active_tab {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let loading_indicator = if tab.is_loading { " ●" } else { "" };
            Line::from(Span::styled(
                format!("{}  {}{}", tab_index_symbol(i), tab.name, loading_indicator),
                style,
            ))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("  ❄️ Yuki")
                .title_style(Style::default().add_modifier(Modifier::DIM)),
        )
        .select(app.active_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" │ ");

    f.render_widget(tabs, area);
}

fn tab_index_symbol(i: usize) -> &'static str {
    match i {
        0 => "一",
        1 => "二",
        2 => "三",
        4 => "五",
        5 => "六",
        6 => "七",
        7 => "八",
        8 => "九",
        9 => "十",
        _ => "・",
    }
}

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    let tab = app.active_tab();

    let content: Vec<Line> = tab
        .messages
        .iter()
        .flat_map(|msg| {
            let (prefix, style) = match msg.role {
                MessageRole::User => ("> ", Style::default().fg(Color::Cyan)),
                MessageRole::Assistant => ("◰ ", Style::default().fg(Color::White)),
                MessageRole::System => ("· ", Style::default().fg(Color::Yellow)),
                MessageRole::ShellOutput => ("$ ", Style::default().fg(Color::Green)),
            };

            vec![Line::from(Span::styled(
                format!("{}{}", prefix, msg.content),
                style,
            ))]
        })
        .collect();

    let paragraph = Paragraph::new(Text::from(content))
        .block(
            Block::default()
                .borders(Borders::NONE)
                .title(format!("  [{}]", tab.name))
                .title_style(Style::default().add_modifier(Modifier::DIM)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);

    if tab.is_loading {
        let loading_area = Rect {
            x: area.x + 1,
            y: area.y + area.height - 1,
            width: 10,
            height: 1,
        };
        let loading = Paragraph::new(Text::from(vec![Line::from(Span::styled(
            "思考中...",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::SLOW_BLINK),
        ))]));
        f.render_widget(loading, loading_area);
    }
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let tab = app.active_tab();
    let input_prefix = if tab.input.starts_with('!') {
        Span::styled("! ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else if tab.input.starts_with('/') {
        Span::styled("/ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        Span::raw("> ")
    };

    let input = Paragraph::new(Line::from(vec![
        input_prefix,
        Span::styled(&tab.input[..tab.cursor_pos.max(1)], Style::default().fg(Color::White)),
        Span::styled("█", Style::default().bg(Color::White).fg(Color::Black)),
        Span::styled(&tab.input[tab.cursor_pos..], Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(input, area);

    f.set_cursor_position((
        area.x + 2 + tab.cursor_pos as u16,
        area.y + 1,
    ));
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let api_status = match app.active_tab().api_key(&app.config) {
        Some(_) => "✓",
        None => "✗",
    };

    let status = format!(
        "  Ctrl+q：退出  │  Tab：切换  │  Alt+1/2/3：跳转  │  Enter：发送  │  {}  │  API: {}  │  {}",
        app.status_message, app.active_tab().name, api_status
    );

    let status_bar = Paragraph::new(Line::from(Span::styled(
        status,
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
    )))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(status_bar, area);
}

fn render_help_popup(f: &mut Frame, area: Rect) {
    let help_area = centered_rect(60, 65, area);
    f.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ❄️ Yuki 帮助  ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("  【命令分流系统】", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  !", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" 后接 Shell 命令 - 直接在终端执行"),
        ]),
        Line::from("     例：!ls -la  查看文件"),
        Line::from("     例：!cargo build  编译项目"),
        Line::from("     例：!gh auth login  GitHub 登录"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  /", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" 后接内部命令 - Yuki 内置功能"),
        ]),
        Line::from("     例：/help  显示帮助"),
        Line::from("     例：/clear 清空对话"),
        Line::from("     例：/config 查看配置"),
        Line::from("     例：/reload 重载配置"),
        Line::from("     例：/add <名> 添加标签"),
        Line::from(""),
        Line::from(Span::raw("  其他输入  - 发送给当前标签的 AI 服务")),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled("  【快捷键】", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  Ctrl+q        退出应用"),
        Line::from("  Tab / S-Tab   切换标签"),
        Line::from("  Alt+1/2/3     跳转到指定标签"),
        Line::from("  ?             显示/隐藏帮助"),
        Line::from("  Enter         发送"),
        Line::from(""),
        Line::from(Span::styled(
            "  ❄️ Yuki - 极简 AI 终端",
            Style::default().add_modifier(Modifier::DIM),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title("  帮助 ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .alignment(ratatui::layout::Alignment::Left);

    f.render_widget(help, help_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// ==================== 主函数 ====================

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("错误：{:?}", err);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_input(key);
                    if app.should_quit {
                        return Ok(());
                    }
                }
            }
        }
    }
}
