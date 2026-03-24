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
        // 简单 TOML 解析（实际项目应用 toml crate）
        ApiConfig {
            claude_api_key: extract_value(&content, "claude_api_key"),
            claude_base_url: extract_value(&content, "claude_base_url"),
            openai_api_key: extract_value(&content, "openai_api_key"),
            custom_api_key: extract_value(&content, "custom_api_key"),
            custom_base_url: extract_value(&content, "custom_base_url"),
        }
    } else {
        // 创建默认配置
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
struct Message {
    role: String,
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
                role: "system".to_string(),
                content: format!("欢迎使用 {}", name),
            }],
            input: String::new(),
            cursor_pos: 0,
            is_loading: false,
            api_type,
        }
    }

    fn api_endpoint(&self, config: &ApiConfig) -> Option<String> {
        match self.api_type {
            ApiType::Claude => config.claude_base_url.clone(),
            ApiType::OpenAI => Some("https://api.openai.com/v1/chat/completions".to_string()),
            ApiType::Custom => config.custom_base_url.clone(),
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

    fn send_message(&mut self) {
        let input = {
            let tab = self.active_tab();
            tab.input.clone()
        };

        if input.is_empty() {
            return;
        }

        // 添加用户消息
        {
            let tab = self.active_tab_mut();
            tab.messages.push(Message {
                role: "user".to_string(),
                content: input.clone(),
            });
            tab.is_loading = true;
            tab.input.clear();
            tab.cursor_pos = 0;
        }

        self.status_message = format!("正在发送请求到 {}...", self.active_tab().name);

        // 模拟 API 响应（实际项目中用 tokio + reqwest）
        let response = simulate_api_response(&input, &self.active_tab().name);

        {
            let tab = self.active_tab_mut();
            tab.messages.push(Message {
                role: "assistant".to_string(),
                content: response,
            });
            tab.is_loading = false;
        }
        self.status_message = "回复已完成".to_string();
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
                // 刷新/重新发送
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
                self.send_message();
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
            Constraint::Length(3),  // 标签栏
            Constraint::Min(0),     // 内容区
            Constraint::Length(3),  // 输入区
            Constraint::Length(1),  // 状态栏
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
            let prefix = match msg.role.as_str() {
                "user" => "  > ",
                "assistant" => "  ◰ ",
                _ => "  · ",
            };

            let style = match msg.role.as_str() {
                "user" => Style::default().fg(Color::Cyan),
                "assistant" => Style::default().fg(Color::White),
                _ => Style::default().fg(Color::DarkGray),
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

    let input = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(&tab.input[..tab.cursor_pos], Style::default().fg(Color::White)),
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
    let help_area = centered_rect(55, 55, area);
    f.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  帮助  ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Ctrl + q      退出应用"),
        Line::from("  Tab / S-Tab   切换标签"),
        Line::from("  Alt + 1/2/3   跳转到指定标签"),
        Line::from("  ?             显示/隐藏帮助"),
        Line::from("  Enter         发送消息"),
        Line::from("  Ctrl + r      刷新状态"),
        Line::from(""),
        Line::from("  配置文件：~/.config/yuki/config.toml"),
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
