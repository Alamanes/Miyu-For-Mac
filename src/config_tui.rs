use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

use crate::config::AppConfig;
use crate::persona::Persona;

const MIYU_CONFIG_FILE: &str = ".miyu_macos.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub default_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiyuLocalConfig {
    #[serde(default = "default_providers")]
    pub providers: Vec<ProviderConfig>,
    pub active_provider: String,
    pub active_model: String,
    #[serde(default)]
    pub show_reasoning: bool,
    #[serde(default = "default_true")]
    pub tools_enabled: bool,
}

fn default_providers() -> Vec<ProviderConfig> {
    vec![ProviderConfig {
        id: "opencode".into(),
        name: "opencode Zen".into(),
        base_url: "https://opencode.ai/zen/v1".into(),
        api_key: String::new(),
        models: vec!["big-pickle".into()],
        default_model: "big-pickle".into(),
    }]
}

fn default_true() -> bool { true }

impl Default for MiyuLocalConfig {
    fn default() -> Self {
        let providers = default_providers();
        Self {
            active_provider: "opencode".into(),
            active_model: "big-pickle".into(),
            providers,
            show_reasoning: false,
            tools_enabled: true,
        }
    }
}

impl MiyuLocalConfig {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        let path = format!("{home}/{MIYU_CONFIG_FILE}");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let home = std::env::var("HOME").unwrap_or_default();
        let path = format!("{home}/{MIYU_CONFIG_FILE}");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, format!("{json}\n"))?;
        Ok(())
    }

    pub fn active_provider(&self) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.id == self.active_provider)
    }

    pub fn active_url(&self) -> String {
        self.active_provider()
            .map(|p| p.base_url.clone())
            .unwrap_or_else(|| "https://opencode.ai/zen/v1".into())
    }

    pub fn active_model_name(&self) -> String {
        if self.active_model.is_empty() {
            self.active_provider()
                .and_then(|p| {
                    if p.default_model.is_empty() {
                        p.models.first().cloned()
                    } else {
                        Some(p.default_model.clone())
                    }
                })
                .unwrap_or_else(|| "big-pickle".into())
        } else {
            self.active_model.clone()
        }
    }
}

pub async fn run() -> Result<()> {
    let cfg = AppConfig::load()?;
    let persona = Persona::build(&cfg);
    let mut local = MiyuLocalConfig::load();
    let changed = TerminalSession::start().await?.run(&cfg, &persona, &mut local).await?;
    if changed {
        local.save()?;
    }
    Ok(())
}

struct TerminalSession {
    stdout: io::Stdout,
}

impl TerminalSession {
    async fn start() -> Result<Self> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)?;
        Ok(Self { stdout })
    }

    async fn run(
        mut self,
        cfg: &AppConfig,
        persona: &Persona,
        local: &mut MiyuLocalConfig,
    ) -> Result<bool> {
        let changed = run_main_menu(&mut self.stdout, cfg, persona, local)?;
        execute!(self.stdout, Show, LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        Ok(changed)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = execute!(self.stdout, Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn run_main_menu(
    stdout: &mut io::Stdout,
    cfg: &AppConfig,
    persona: &Persona,
    local: &mut MiyuLocalConfig,
) -> Result<bool> {
    let mut selected = 0usize;
    let mut changed = false;

    loop {
        let reasoning_status = if local.show_reasoning { "开" } else { "关" };
        let tools_status = if local.tools_enabled { "开" } else { "关" };
        let active_label = format!("{} / {}", local.active_provider, local.active_model_name());

        let options = [
            format!("当前模型: {active_label}"),
            format!("管理供应商 ({} 个)", local.providers.len()),
            format!("显示思考过程: {reasoning_status}"),
            format!("工具调用: {tools_status}"),
            format!("人格: {} (来自 {})", persona.persona_name, persona.persona_name),
            format!("Shell: {} (来自 opencode.json)", persona.workdir),
            "保存并退出".to_string(),
            "不保存退出".to_string(),
        ];

        let help = if selected == 0 {
            "[Enter]切换模型 [j/k]导航 [s]保存 [q]退出"
        } else if selected == 1 {
            "[Enter]管理供应商 [j/k]导航 [q]退出"
        } else if selected < 4 {
            "[Space]切换 [j/k]导航 [s]保存 [q]退出"
        } else {
            "[Enter]查看 [j/k]导航 [s]保存 [q]退出"
        };

        draw_menu(stdout, " MIYU-MACOS ", &options, selected, help)?;

        match read_key()? {
            KeyCode::Char('q') | KeyCode::Esc => {
                if selected == options.len() - 1 {
                    return Ok(changed);
                }
                return Ok(false);
            }
            KeyCode::Char('s') => {
                local.save()?;
                return Ok(true);
            }
            KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                selected = (selected + 1).min(options.len() - 1)
            }
            KeyCode::Char(' ') => match selected {
                2 => { local.show_reasoning = !local.show_reasoning; changed = true; }
                3 => { local.tools_enabled = !local.tools_enabled; changed = true; }
                _ => {}
            },
            KeyCode::Enter => match selected {
                0 => { select_model(stdout, local)?; changed = true; }
                1 => { manage_providers(stdout, local)?; changed = true; }
                4 => show_persona_detail(stdout, persona)?,
                5 => show_miyu_config(stdout, cfg)?,
                6 => { local.save()?; return Ok(true); }
                7 => return Ok(changed),
                _ => {}
            },
            _ => {}
        }
    }
}

fn select_model(stdout: &mut io::Stdout, local: &mut MiyuLocalConfig) -> Result<()> {
    let mut selected = 0usize;
    loop {
        let mut options: Vec<String> = Vec::new();
        for provider in &local.providers {
            options.push(format!("══ {} ══", provider.name));
            let models = if provider.models.is_empty() {
                vec![provider.default_model.clone()]
            } else {
                provider.models.clone()
            };
            for model in &models {
                let marker = if provider.id == local.active_provider
                    && (model == &local.active_model_name()
                        || (local.active_model.is_empty()
                            && Some(model.as_str()) == models.first().map(|s| s.as_str())))
                {
                    "● "
                } else {
                    "  "
                };
                options.push(format!("{marker}{model}"));
            }
        }

        draw_menu(
            stdout,
            " 选择模型 ",
            &options,
            selected,
            "[j/k]移动 [Enter]激活 [q]返回",
        )?;

        match read_key()? {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                selected = (selected + 1).min(options.len().saturating_sub(1))
            }
            KeyCode::Enter => {
                let (provider_id, model) = resolve_selection(&options, selected);
                if !model.is_empty() {
                    local.active_provider = provider_id.to_string();
                    local.active_model = model.to_string();
                    return Ok(());
                }
            }
            _ => {}
        }
    }
}

fn resolve_selection(options: &[String], index: usize) -> (String, String) {
    let mut current_provider = String::new();
    let mut skip = 0usize;
    for (i, opt) in options.iter().enumerate() {
        if opt.starts_with("══") {
            current_provider = String::new();
            skip = i;
            continue;
        }
        if i == index {
            let clean = opt.trim_start_matches("● ").trim_start_matches("  ");
            if current_provider.is_empty() {
                for j in (0..i).rev() {
                    if options[j].starts_with("══") {
                        current_provider = options[j]
                            .trim_start_matches("══ ")
                            .trim_end_matches(" ══")
                            .to_string();
                        break;
                    }
                }
            }
            let provider_id = find_provider_id_by_name(options, &current_provider, skip);
            return (provider_id, clean.to_string());
        }
    }
    (String::new(), String::new())
}

fn find_provider_id_by_name(options: &[String], name: &str, from: usize) -> String {
    if let Some(opt) = options.get(from) {
        let raw = opt.trim_start_matches("══ ").trim_end_matches(" ══");
        if raw == name {
            return raw.to_lowercase().replace(' ', "-");
        }
    }
    name.to_lowercase().replace(' ', "-")
}

fn manage_providers(stdout: &mut io::Stdout, local: &mut MiyuLocalConfig) -> Result<()> {
    let mut selected = 0usize;
    loop {
        let mut options: Vec<String> = local
            .providers
            .iter()
            .map(|p| {
                let marker = if p.id == local.active_provider { "● " } else { "  " };
                format!("{marker}{} ({})", p.name, p.base_url)
            })
            .collect();
        options.push("  [+ 添加供应商]".to_string());

        draw_menu(
            stdout,
            " 供应商管理 ",
            &options,
            selected,
            "[Enter]编辑 [d]删除 [a]添加 [Tab]设为活跃 [q]返回",
        )?;

        match read_key()? {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                selected = (selected + 1).min(options.len().saturating_sub(1))
            }
            KeyCode::Tab => {
                if selected < local.providers.len() {
                    local.active_provider = local.providers[selected].id.clone();
                    local.active_model.clear();
                }
            }
            KeyCode::Char('a') => {
                edit_provider_form(stdout, local, None)?;
            }
            KeyCode::Char('d') => {
                if selected < local.providers.len() && local.providers.len() > 1 {
                    let removed = local.providers.remove(selected);
                    if local.active_provider == removed.id {
                        local.active_provider = local.providers[0].id.clone();
                        local.active_model.clear();
                    }
                    selected = selected.saturating_sub(1);
                }
            }
            KeyCode::Enter => {
                if selected < local.providers.len() {
                    let provider = local.providers[selected].clone();
                    edit_provider_form(stdout, local, Some(&provider))?;
                } else {
                    edit_provider_form(stdout, local, None)?;
                }
            }
            _ => {}
        }
    }
}

fn edit_provider_form(
    stdout: &mut io::Stdout,
    local: &mut MiyuLocalConfig,
    existing: Option<&ProviderConfig>,
) -> Result<()> {
    let is_new = existing.is_none();
    let title = if is_new { " 添加供应商 " } else { " 编辑供应商 " };

    let mut id = existing.map(|p| p.id.clone()).unwrap_or_default();
    let mut name = existing.map(|p| p.name.clone()).unwrap_or_default();
    let mut base_url = existing.map(|p| p.base_url.clone()).unwrap_or_default();
    let mut api_key = existing.map(|p| p.api_key.clone()).unwrap_or_default();
    let mut models_str = existing
        .map(|p| p.models.join(", "))
        .unwrap_or_default();

    let field_labels = ["供应商 ID (英文标识)", "显示名称", "API Base URL", "API Key (留空则不认证)", "模型列表 (逗号分隔)"];
    let field_sensitive = [false, false, false, true, false];

    let mut field_idx = 0usize;
    loop {
        let (cols, rows) = terminal::size()?;
        let w = cols.saturating_sub(4).max(50);
        let h = 14u16;
        let x = 2u16;
        let y = (rows.saturating_sub(h)) / 2;

        let values: [&str; 5] = [
            &id, &name, &base_url,
            if api_key.is_empty() { "" } else { "***隐藏***" },
            &models_str,
        ];

        queue!(stdout, Clear(ClearType::All))?;
        draw_box(stdout, x, y, w, h, title)?;

        for i in 0..field_labels.len() {
            let display_val = if field_sensitive[i] && values[i] == "***隐藏***" {
                values[i]
            } else if values[i].len() > w.saturating_sub(20) as usize {
                &values[i][..w.saturating_sub(23) as usize]
            } else {
                values[i]
            };
            let prefix = if i == field_idx { "> " } else { "  " };
            queue!(
                stdout,
                MoveTo(x + 2, y + 2 + i as u16),
                Print(format!("{prefix}{}: {display_val}", field_labels[i]))
            )?;
        }

        let save_line = y + 2 + field_labels.len() as u16;
        queue!(
            stdout,
            MoveTo(x + 2, save_line),
            Print("[Enter]下一项 [Tab]保存 [Esc]取消 [Backspace]编辑当前值")
        )?;

        stdout.flush()?;

        match read_key()? {
            KeyCode::Esc => return Ok(()),
            KeyCode::Tab => {
                if id.trim().is_empty() { continue; }
                let models: Vec<String> = models_str
                    .split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                let dm = models.first().cloned().unwrap_or_default();

                let id_save = id.clone();
                let name_save = name.clone();
                let url_save = base_url.trim_end_matches('/').to_string();
                let key_save = api_key.clone();
                let models_save = models.clone();

                if let Some(existing) = existing {
                    if let Some(p) = local.providers.iter_mut().find(|p| p.id == existing.id) {
                        p.id = id_save.clone(); p.name = name_save;
                        p.base_url = url_save; p.api_key = key_save;
                        p.models = models_save; p.default_model = dm;
                    }
                    if local.active_provider == existing.id { local.active_provider = id_save; }
                } else {
                    local.providers.push(ProviderConfig {
                        id: id_save, name: name_save, base_url: url_save,
                        api_key: key_save, models: models_save, default_model: dm,
                    });
                }
                return Ok(());
            }
            KeyCode::Enter => { field_idx = (field_idx + 1) % field_labels.len(); }
            KeyCode::Up | KeyCode::Char('k') => { field_idx = field_idx.saturating_sub(1); }
            KeyCode::Down | KeyCode::Char('j') => {
                field_idx = (field_idx + 1).min(field_labels.len() - 1);
            }
            KeyCode::Backspace => {
                let current = match field_idx {
                    0 => &id, 1 => &name, 2 => &base_url, 3 => &api_key, 4 => &models_str,
                    _ => continue,
                };
                let new_val = edit_line(stdout, field_labels[field_idx], current)?;
                match field_idx {
                    0 => id = new_val, 1 => name = new_val, 2 => base_url = new_val,
                    3 => api_key = new_val, 4 => models_str = new_val,
                    _ => {}
                }
            }
            KeyCode::Char(c) => {
                let current = match field_idx {
                    0 => &id, 1 => &name, 2 => &base_url, 3 => &api_key, 4 => &models_str,
                    _ => continue,
                };
                let new_val = edit_line_starting_with(stdout, field_labels[field_idx], current, c)?;
                match field_idx {
                    0 => id = new_val, 1 => name = new_val, 2 => base_url = new_val,
                    3 => api_key = new_val, 4 => models_str = new_val,
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn edit_line(stdout: &mut io::Stdout, label: &str, current: &str) -> Result<String> {
    edit_line_starting_with(stdout, label, current, '\0')
}

fn edit_line_starting_with(
    stdout: &mut io::Stdout,
    label: &str,
    current: &str,
    first_char: char,
) -> Result<String> {
    let (cols, rows) = terminal::size()?;
    let w = cols.saturating_sub(4).max(40);
    let h = 8u16;
    let x = 2u16;
    let y = (rows.saturating_sub(h)) / 2;

    let mut input = if first_char == '\0' {
        current.to_string()
    } else {
        format!("{}{}", current, first_char)
    };
    let mut cursor = input.len();

    loop {
        queue!(stdout, Clear(ClearType::All))?;
        draw_box(stdout, x, y, w, h, &format!(" 编辑: {label} "))?;

        let display = if input.len() > w.saturating_sub(6) as usize {
            let start = input.len().saturating_sub(w.saturating_sub(8) as usize);
            format!("...{}", &input[start..])
        } else {
            input.clone()
        };

        queue!(stdout, MoveTo(x + 2, y + 2), Print("> "), Print(&display))?;
        queue!(
            stdout,
            MoveTo(x + 2, y + 4),
            SetAttribute(Attribute::Dim),
            Print("[Enter]确认 [Esc]取消"),
            SetAttribute(Attribute::Reset)
        )?;
        stdout.flush()?;

        match read_key()? {
            KeyCode::Esc => return Ok(current.to_string()),
            KeyCode::Enter => return Ok(input),
            KeyCode::Backspace => {
                if cursor > 0 {
                    cursor -= 1;
                    input.remove(cursor);
                }
            }
            KeyCode::Left => cursor = cursor.saturating_sub(1),
            KeyCode::Right => cursor = (cursor + 1).min(input.len()),
            KeyCode::Char(c) => {
                input.insert(cursor, c);
                cursor += 1;
            }
            _ => {}
        }
    }
}

fn show_persona_detail(stdout: &mut io::Stdout, persona: &Persona) -> Result<()> {
    let mut lines = vec![
        format!("人格: {}", persona.persona_name),
        format!("用户: {}", persona.user_name),
        format!("来源: ~/.miyu/persona.md"),
        format!("可通过 MIYU_PERSONA 环境变量自定义路径"),
        String::new(),
        "System Prompt:".to_string(),
        String::new(),
    ];
    for line in persona.system_prompt.lines() {
        lines.push(line.to_string());
    }
    lines.push(String::new());
    lines.push("按任意键返回...".to_string());
    show_scrollable(stdout, " 人格详情 ", &lines)?;
    Ok(())
}

fn show_miyu_config(stdout: &mut io::Stdout, cfg: &AppConfig) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_default();
    let lines = vec![
        format!("工作目录: {}", cfg.workdir),
        format!("人格文件: {}", cfg.persona_file),
        String::new(),
        "环境变量:".to_string(),
        "  MIYU_WORKDIR   工作目录 (默认: 当前目录)".to_string(),
        "  MIYU_PERSONA   人格文件路径 (默认: ~/.miyu/persona.md)".to_string(),
        String::new(),
        "LLM 供应商/模型配置: {}/.miyu_macos.json".to_string(),
        String::new(),
        "按任意键返回...".to_string(),
    ];
    show_scrollable(stdout, " Miyu 配置 ", &lines)?;
    Ok(())
}

fn show_scrollable(stdout: &mut io::Stdout, title: &str, lines: &[String]) -> Result<()> {
    let (cols, rows) = terminal::size()?;
    let width = cols.saturating_sub(4).max(40);
    let height = rows.saturating_sub(2).max(10);
    let x = 2u16;
    let y = 1u16;
    let visible = height.saturating_sub(4) as usize;
    let total = lines.len();
    let mut scroll = 0usize;

    loop {
        queue!(stdout, Clear(ClearType::All))?;
        draw_box(stdout, x, y, width, height, title)?;

        let end = (scroll + visible).min(total);
        for (i, line) in lines[scroll..end].iter().enumerate() {
            let display = truncate(line, width.saturating_sub(6) as usize);
            queue!(stdout, MoveTo(x + 2, y + 2 + i as u16), Print(display))?;
        }

        if total > visible {
            let pct = (scroll * 100) / (total.saturating_sub(visible).max(1));
            queue!(
                stdout,
                MoveTo(x + 2, y + height - 1),
                Print(format!(" [↑↓滚动] {pct}% [任意键返回]"))
            )?;
        }

        stdout.flush()?;
        match read_key()? {
            KeyCode::Up | KeyCode::Char('k') => scroll = scroll.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                scroll = (scroll + 1).min(total.saturating_sub(visible))
            }
            _ => return Ok(()),
        }
    }
}

fn draw_menu(
    stdout: &mut io::Stdout,
    title: &str,
    options: &[String],
    selected: usize,
    help: &str,
) -> Result<()> {
    let (cols, rows) = terminal::size()?;
    let width = cols.saturating_sub(4).max(50);
    let height = rows.saturating_sub(2).max(16);
    let x = 2u16;
    let y = 1u16;

    queue!(stdout, Clear(ClearType::All))?;
    draw_box(stdout, x, y, width, height, title)?;

    let visible = height.saturating_sub(4) as usize;
    let start = selected.saturating_sub(visible.saturating_sub(1));
    let end = (start + visible).min(options.len());

    for (i, option) in options[start..end].iter().enumerate() {
        let index = start + i;
        let row = y + 2 + i as u16;
        let display = truncate(option, width.saturating_sub(6) as usize);

        queue!(stdout, MoveTo(x + 2, row))?;
        if index == selected {
            queue!(
                stdout,
                SetAttribute(Attribute::Reverse),
                Print(format!(" {:<pad$} ", display, pad = width.saturating_sub(6) as usize)),
                SetAttribute(Attribute::Reset)
            )?;
        } else {
            queue!(stdout, Print(format!("  {display}")))?;
        }
    }

    queue!(
        stdout,
        MoveTo(x + 2, y + height - 1),
        SetAttribute(Attribute::Dim),
        Print(help),
        SetAttribute(Attribute::Reset)
    )?;

    stdout.flush()?;
    Ok(())
}

fn draw_box(
    stdout: &mut io::Stdout,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    title: &str,
) -> Result<()> {
    let w = width.max(2);
    let h = height.max(2);
    let top = "─".repeat(w.saturating_sub(2) as usize);
    let bot = "─".repeat(w.saturating_sub(2) as usize);

    queue!(stdout, MoveTo(x, y), Print("┌"), Print(&top), Print("┐"))?;

    let title_offset = 2usize;
    if title_offset + title.len() < top.len() {
        queue!(stdout, MoveTo(x + title_offset as u16, y), Print(title))?;
    }

    for row in 1..h - 1 {
        queue!(stdout, MoveTo(x, y + row), Print("│"))?;
        queue!(stdout, MoveTo(x + w - 1, y + row), Print("│"))?;
    }

    queue!(stdout, MoveTo(x, y + h - 1), Print("└"), Print(&bot), Print("┘"))?;
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}...", &s[..max.saturating_sub(3)]) }
}

fn read_key() -> Result<KeyCode> {
    loop {
        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            return Ok(code);
        }
    }
}
