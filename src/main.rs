use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap, List, ListItem},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, stdout},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
    path::PathBuf,
};

// ==================== 配置结构 ====================

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct ApiConfig {
    claude_api_key: Option<String>,
    claude_base_url: Option<String>,
    openai_api_key: Option<String>,
    custom_api_key: Option<String>,
    custom_base_url: Option<String>,
}

impl ApiConfig {
    fn load() -> Self {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("tabby")
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
            let _ = fs::write(&config_path, "# Tabby 🐱 配置文件\n");
            config
        }
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

#[derive(Clone, Debug, Serialize, Deserialize)]
enum MessageRole {
    User,
    Assistant,
    System,
    ShellOutput,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Message {
    role: MessageRole,
    content: String,
    timestamp: u64,
}

impl Message {
    fn new(role: MessageRole, content: String) -> Self {
        Self {
            role,
            content,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

// ==================== 项目结构 ====================

#[derive(Clone, Debug, Serialize, Deserialize)]
enum ProjectStatus {
    Active,
    Paused,
    Archived,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DailySummary {
    date: String,
    events: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Todo {
    content: String,
    done: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Project {
    name: String,
    status: ProjectStatus,
    created_at: u64,
    last_active: u64,
    messages: Vec<Message>,
    todos: Vec<Todo>,
    daily_summaries: Vec<DailySummary>,
    input: String,
    cursor_pos: usize,
}

impl Project {
    fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            name: name.to_string(),
            status: ProjectStatus::Active,
            created_at: now,
            last_active: now,
            messages: vec![Message::new(
                MessageRole::System,
                format!("项目「{}」已创建", name),
            )],
            todos: vec![],
            daily_summaries: vec![],
            input: String::new(),
            cursor_pos: 0,
        }
    }

    fn save(&self, base_path: &PathBuf) -> io::Result<()> {
        let project_path = base_path.join(&self.name);
        fs::create_dir_all(&project_path)?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(project_path.join("project.json"), content)?;
        Ok(())
    }

    fn load(name: &str, base_path: &PathBuf) -> io::Result<Option<Self>> {
        let project_path = base_path.join(name);
        let file_path = project_path.join("project.json");
        if file_path.exists() {
            let content = fs::read_to_string(&file_path)?;
            Ok(serde_json::from_str(&content).ok())
        } else {
            Ok(None)
        }
    }
}

// ==================== 应用状态 ====================

struct App {
    projects: Vec<Project>,
    active_project: usize,
    should_quit: bool,
    show_help: bool,
    show_monitor: bool,
    config: ApiConfig,
    status_message: String,
    projects_path: PathBuf,
}

impl App {
    fn new() -> Self {
        let config = ApiConfig::load();
        let projects_path = dirs::config_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("tabby")
            .join("projects");

        fs::create_dir_all(&projects_path).ok();

        // 加载现有项目
        let mut projects = Vec::new();
        if let Ok(entries) = fs::read_dir(&projects_path) {
            for entry in entries.flatten() {
                if let Ok(project) = Project::load(&entry.file_name().to_string_lossy(), &projects_path) {
                    if let Some(p) = project {
                        projects.push(p);
                    }
                }
            }
        }

        // 如果没有项目，创建默认项目
        if projects.is_empty() {
            projects.push(Project::new("默认项目"));
        }

        Self {
            projects,
            active_project: 0,
            should_quit: false,
            show_help: false,
            show_monitor: false,
            config,
            status_message: String::from("准备就绪"),
            projects_path,
        }
    }

    fn active_project(&self) -> &Project {
        &self.projects[self.active_project]
    }

    fn create_project(&mut self, name: &str) {
        let project = Project::new(name);
        let _ = project.save(&self.projects_path);
        self.projects.push(project);
        self.active_project = self.projects.len() - 1;
        self.status_message = format!("项目「{}」已创建", name);
    }

    fn save_current_project(&mut self) {
        let (project_path, project) = (&self.projects_path, &mut self.projects[self.active_project]);
        project.last_active = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let _ = project.save(project_path);
    }

    fn run_shell_command(&mut self, cmd: &str) {
        self.status_message = format!("执行：!{}", cmd);

        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output();

        let msg = match output {
            Ok(out) => {
                let result = if !out.stdout.is_empty() {
                    String::from_utf8_lossy(&out.stdout).to_string()
                } else if !out.stderr.is_empty() {
                    String::from_utf8_lossy(&out.stderr).to_string()
                } else {
                    "命令执行成功（无输出）".to_string()
                };
                self.status_message = "命令执行完成".to_string();
                Message::new(MessageRole::ShellOutput, format!("$ {}\n{}", cmd, result))
            }
            Err(e) => {
                self.status_message = "命令执行失败".to_string();
                Message::new(MessageRole::ShellOutput, format!("$ {}\n错误：{}", cmd, e))
            }
        };

        let project = &mut self.projects[self.active_project];
        project.messages.push(msg);
    }

    fn run_internal_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let command = parts.get(0).unwrap_or(&"");

        let msg = match *command {
            "help" | "?" => {
                self.show_help = true;
                return;
            }
            "clear" => {
                self.status_message = "对话已清空".to_string();
                Message::new(MessageRole::System, "对话已清空".to_string())
            }
            "monitor" => {
                self.show_monitor = !self.show_monitor;
                return;
            }
            "new" | "create" => {
                if parts.len() > 1 {
                    let name = parts[1];
                    self.create_project(name);
                } else {
                    self.status_message = "用法：/new <项目名>".to_string();
                }
                return;
            }
            "list" => {
                self.status_message = "项目列表已显示".to_string();
                let list: Vec<String> = self.projects.iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let marker = if i == self.active_project { "●" } else { "○" };
                        format!("{} {}", marker, p.name)
                    })
                    .collect();
                Message::new(MessageRole::System, format!("项目列表:\n{}", list.join("\n")))
            }
            "config" => {
                self.status_message = "配置信息已显示".to_string();
                let claude_status = if self.config.claude_api_key.is_some() { "已配置" } else { "未配置" };
                let openai_status = if self.config.openai_api_key.is_some() { "已配置" } else { "未配置" };
                Message::new(
                    MessageRole::System,
                    format!(
                        "配置文件：~/.config/tabby/config.toml\n\
                         Claude API: {}\n\
                         OpenAI API: {}",
                        claude_status, openai_status
                    ),
                )
            }
            _ => {
                self.status_message = format!("未知命令：/{}", command);
                return;
            }
        };

        let project = &mut self.projects[self.active_project];
        project.messages.push(msg);
    }

    fn send_to_ai(&mut self, input: &str) {
        let project_name = self.projects[self.active_project].name.clone();
        self.status_message = format!("正在发送请求到 {}...", project_name);

        let response = format!("[{}] 收到：{}\n\n这是一条模拟回复。", project_name, input);

        let project = &mut self.projects[self.active_project];
        project.messages.push(Message::new(MessageRole::User, input.to_string()));
        project.messages.push(Message::new(MessageRole::Assistant, response));
        self.status_message = "回复已完成".to_string();

        self.save_current_project();
    }

    fn process_input(&mut self) {
        let input = {
            let project = &self.projects[self.active_project];
            project.input.clone()
        };

        if input.is_empty() {
            return;
        }

        if input.starts_with('!') {
            let cmd = input.trim_start_matches('!').trim();
            self.run_shell_command(cmd);
        } else if input.starts_with('/') {
            let cmd = input.trim_start_matches('/').trim();
            self.run_internal_command(cmd);
        } else {
            self.send_to_ai(&input);
        }

        let project = &mut self.projects[self.active_project];
        project.input.clear();
        project.cursor_pos = 0;
    }

    fn handle_input(&mut self, key: event::KeyEvent) {
        // 监测面板模式
        if self.show_monitor {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.show_monitor = false;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.active_project < self.projects.len() - 1 {
                        self.active_project += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if self.active_project > 0 {
                        self.active_project -= 1;
                    }
                }
                KeyCode::Enter => {
                    self.show_monitor = false;
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.previous_project();
                } else {
                    self.next_project();
                }
            }
            KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.show_monitor = !self.show_monitor;
            }
            KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::ALT) => {
                if self.projects.len() > 0 {
                    self.active_project = 0;
                }
            }
            KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::ALT) => {
                if self.projects.len() > 1 {
                    self.active_project = 1;
                }
            }
            KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::ALT) => {
                if self.projects.len() > 2 {
                    self.active_project = 2;
                }
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.create_project(&format!("项目{}", self.projects.len() + 1));
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char(c) => {
                let project = &mut self.projects[self.active_project];
                project.input.insert(project.cursor_pos, c);
                project.cursor_pos += 1;
            }
            KeyCode::Backspace => {
                let project = &mut self.projects[self.active_project];
                if project.cursor_pos > 0 {
                    project.cursor_pos -= 1;
                    project.input.remove(project.cursor_pos);
                }
            }
            KeyCode::Enter => {
                self.process_input();
            }
            KeyCode::Left => {
                let project = &mut self.projects[self.active_project];
                if project.cursor_pos > 0 {
                    project.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                let project = &mut self.projects[self.active_project];
                if project.cursor_pos < project.input.len() {
                    project.cursor_pos += 1;
                }
            }
            _ => {}
        }
    }

    fn next_project(&mut self) {
        self.active_project = (self.active_project + 1) % self.projects.len();
    }

    fn previous_project(&mut self) {
        self.active_project = (self.active_project + self.projects.len() - 1) % self.projects.len();
    }
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

    if app.show_monitor {
        render_monitor(f, app);
    }
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, project)| {
            let style = if i == app.active_project {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let status = if i == app.active_project { "●" } else { "○" };
            Line::from(Span::styled(
                format!("{} {}", status, project.name),
                style,
            ))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray))
                .title("  🐱 Tabby")
                .title_style(Style::default().add_modifier(Modifier::DIM)),
        )
        .select(app.active_project)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" │ ");

    f.render_widget(tabs, area);
}

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    let project = app.active_project();

    let content: Vec<Line> = project
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
                .title(format!("  [{}]", project.name))
                .title_style(Style::default().add_modifier(Modifier::DIM)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let project = app.active_project();
    let input_prefix = if project.input.starts_with('!') {
        Span::styled("! ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else if project.input.starts_with('/') {
        Span::styled("/ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        Span::raw("> ")
    };

    let cursor_pos = project.cursor_pos.max(1).min(project.input.len());
    let input = Paragraph::new(Line::from(vec![
        input_prefix,
        Span::styled(&project.input[..cursor_pos], Style::default().fg(Color::White)),
        Span::styled("█", Style::default().bg(Color::White).fg(Color::Black)),
        Span::styled(&project.input[cursor_pos..], Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(input, area);

    f.set_cursor_position((
        area.x + 2 + cursor_pos as u16,
        area.y + 1,
    ));
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = format!(
        "  Ctrl+q：退出  │  Tab：切换  │  Ctrl+n：新建  │  Ctrl+m：监测  │  {}",
        app.status_message
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

fn render_monitor(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 70, f.area());
    f.render_widget(Clear, area);

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let items: Vec<ListItem> = app.projects.iter().enumerate().map(|(i, p)| {
        let status_icon = match p.status {
            ProjectStatus::Active => "●",
            ProjectStatus::Paused => "○",
            ProjectStatus::Archived => "◎",
        };
        let active_marker = if i == app.active_project { " ►" } else { "" };

        let events: Vec<String> = p.daily_summaries
            .iter()
            .filter(|s| s.date == today)
            .flat_map(|s| s.events.iter().cloned())
            .collect();

        let events_text = if events.is_empty() {
            "  今日无记录".to_string()
        } else {
            events.iter().map(|e| format!("  · {}", e)).collect::<Vec<_>>().join("\n")
        };

        let content = format!(
            "{}{} {}\n{}",
            status_icon,
            active_marker,
            p.name,
            events_text
        );
        ListItem::new(content)
    }).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("  📊 今日进度 ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn render_help_popup(f: &mut Frame, area: Rect) {
    let help_area = centered_rect(60, 65, area);
    f.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  🐱 Tabby 帮助  ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  【项目隔离】每个项目独立上下文，切换=切换记忆"),
        Line::from(""),
        Line::from("  【快捷键】"),
        Line::from("  Ctrl+q        退出"),
        Line::from("  Tab / S-Tab   切换项目"),
        Line::from("  Ctrl+n        新建项目"),
        Line::from("  Ctrl+m        监测面板"),
        Line::from("  Alt+1/2/3     跳转项目"),
        Line::from("  Enter         发送"),
        Line::from(""),
        Line::from("  【命令】"),
        Line::from("  !ls           Shell 命令"),
        Line::from("  /new 项目名    新建项目"),
        Line::from("  /list         项目列表"),
        Line::from("  /clear        清空对话"),
        Line::from(""),
        Line::from(Span::styled(
            "  🐱 Tabby - 你的终端 AI 工作台",
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
        .alignment(Alignment::Left);

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
