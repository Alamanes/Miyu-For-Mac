use std::process::Command;

use crate::llm::ToolDefinition;

pub struct ToolRegistry {}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {}
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition::new(
                "run_command",
                "在 macOS 终端执行 Shell 命令",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "要执行的 Shell 命令"},
                        "workdir": {"type": "string", "description": "工作目录，默认沙箱"}
                    },
                    "required": ["command"]
                }),
            ),
            ToolDefinition::new(
                "read_file",
                "读取文件内容，返回带行号的文本",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "文件路径"},
                        "offset": {"type": "integer", "description": "起始行"},
                        "limit": {"type": "integer", "description": "读取行数"}
                    },
                    "required": ["path"]
                }),
            ),
            ToolDefinition::new(
                "write_file",
                "将内容写入文件，仅允许写入沙箱目录",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "文件路径（相对于沙箱或绝对路径）"},
                        "content": {"type": "string", "description": "要写入的内容"}
                    },
                    "required": ["path", "content"]
                }),
            ),
            ToolDefinition::new(
                "edit_file",
                "在文件中进行字符串替换",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "文件路径"},
                        "old_string": {"type": "string", "description": "要被替换的文本"},
                        "new_string": {"type": "string", "description": "替换后的文本"}
                    },
                    "required": ["path", "old_string", "new_string"]
                }),
            ),
            ToolDefinition::new(
                "web_search",
                "使用 DuckDuckGo 搜索网页",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "搜索关键词"}
                    },
                    "required": ["query"]
                }),
            ),
            ToolDefinition::new(
                "web_fetch",
                "抓取指定 URL 的内容",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "description": "网页 URL"}
                    },
                    "required": ["url"]
                }),
            ),
            ToolDefinition::new(
                "list_directory",
                "列出目录中的文件和子目录",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "目录路径"}
                    },
                    "required": ["path"]
                }),
            ),
            ToolDefinition::new(
                "search_files",
                "按文件名模式搜索文件",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "文件名模式（glob）"},
                        "directory": {"type": "string", "description": "搜索目录"}
                    },
                    "required": ["pattern"]
                }),
            ),
            ToolDefinition::new(
                "search_content",
                "使用正则表达式搜索文件内容",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "正则表达式"},
                        "directory": {"type": "string", "description": "搜索目录"}
                    },
                    "required": ["pattern"]
                }),
            ),
            ToolDefinition::new(
                "system_info",
                "获取 macOS 系统信息",
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
            ToolDefinition::new(
                "weather",
                "查询城市天气（使用 wttr.in）",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "city": {"type": "string", "description": "城市名称"}
                    },
                    "required": ["city"]
                }),
            ),
            ToolDefinition::new(
                "calculator",
                "安全计算数学表达式",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "expression": {"type": "string", "description": "数学表达式"}
                    },
                    "required": ["expression"]
                }),
            ),
            ToolDefinition::new(
                "brew_info",
                "查询 Homebrew 软件包信息",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "package": {"type": "string", "description": "软件包名称"}
                    },
                    "required": ["package"]
                }),
            ),
            ToolDefinition::new(
                "todowrite",
                "写入/更新当前任务的待办列表",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "todos": {"type": "string", "description": "JSON 数组: [{\"content\":\"...\",\"status\":\"pending|in_progress|completed\",\"priority\":\"high|medium|low\"}]"}
                    },
                    "required": ["todos"]
                }),
            ),
            ToolDefinition::new(
                "ask_question",
                "向用户提问以获取澄清",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "questions": {"type": "string", "description": "JSON 问题数组"}
                    },
                    "required": ["questions"]
                }),
            ),
        ]
    }

    pub async fn call(&self, name: &str, arguments: &str) -> String {
        match name {
            "run_command" => run_command(arguments).await,
            "read_file" => read_file(arguments),
            "write_file" => write_file(arguments),
            "edit_file" => edit_file(arguments),
            "web_search" => web_search(arguments).await,
            "web_fetch" => web_fetch(arguments).await,
            "list_directory" => list_directory(arguments),
            "search_files" => search_files(arguments),
            "search_content" => search_content(arguments),
            "system_info" => system_info(),
            "weather" => weather(arguments).await,
            "calculator" => calculator(arguments),
            "brew_info" => brew_info(arguments),
            "todowrite" => todowrite(arguments),
            "ask_question" => ask_question(),
            _ => format!("unknown tool: {name}"),
        }
    }
}

fn parse_args(args: &str) -> serde_json::Value {
    serde_json::from_str(args).unwrap_or_default()
}

fn sandbox() -> String {
    std::env::var("MIYU_WORKDIR")
        .or_else(|_| std::env::current_dir().map(|p| p.to_string_lossy().to_string()))
        .unwrap_or_else(|_| ".".to_string())
}

fn safe_path(path: &str) -> String {
    let path = path.trim();
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{}/{path}", sandbox())
    }
}

fn is_in_sandbox(path: &str) -> bool {
    let path = std::path::Path::new(path);
    let sb = sandbox();
    let sandbox_path = std::path::Path::new(&sb);
    match path.canonicalize() {
        Ok(canon) => canon.starts_with(sandbox_path),
        Err(_) => {
            if path.is_relative() && !path.starts_with("..") {
                return true;
            }
            if path.is_absolute() && path.to_string_lossy().starts_with(&sb) {
                return true;
            }
            false
        }
    }
}

static DANGEROUS: &[&str] = &[
    "rm -rf /",
    "sudo ",
    "chmod 777 /",
    "mkfs",
    "dd if=",
    ":(){ :|:& };:",
    "> /dev/sda",
];

fn is_dangerous(cmd: &str) -> bool {
    DANGEROUS.iter().any(|p| cmd.contains(p))
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect()
}

async fn run_command(args: &str) -> String {
    let v = parse_args(args);
    let cmd_str = v["command"].as_str().unwrap_or("");
    if cmd_str.is_empty() {
        return "tool error: no command provided".to_string();
    }
    if is_dangerous(cmd_str) {
        return "tool error: dangerous command blocked".to_string();
    }
    let sb = sandbox();
    let wd = v["workdir"].as_str().unwrap_or(&sb);
    match Command::new("fish")
        .args(["-c", cmd_str])
        .current_dir(wd)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(stdout.trim());
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str("[stderr]\n");
                result.push_str(stderr.trim());
            }
            if result.is_empty() {
                result = "(no output)".to_string();
            }
            if result.len() > 4000 {
                result.truncate(4000);
                result.push_str("\n...(truncated)");
            }
            result
        }
        Err(e) => format!("tool error: {e}"),
    }
}

fn read_file(args: &str) -> String {
    let v = parse_args(args);
    let path_str = v["path"].as_str().unwrap_or("");
    if path_str.is_empty() {
        return "tool error: no path provided".to_string();
    }
    let path = safe_path(path_str);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => return format!("tool error: cannot read {path}: {e}"),
    };
    let offset = v["offset"].as_u64().unwrap_or(1).max(1) as usize;
    let limit = v["limit"].as_u64().map(|n| n as usize);
    let lines: Vec<&str> = content.lines().collect();
    let start = (offset - 1).min(lines.len());
    let end = match limit {
        Some(n) => (start + n).min(lines.len()),
        None => lines.len(),
    };
    if start >= lines.len() {
        return format!("(file has {} lines, offset {offset} is out of range)", lines.len());
    }
    let mut result = String::new();
    for (i, line) in lines[start..end].iter().enumerate() {
        result.push_str(&format!("{:>6}: {}\n", start + i + 1, line));
    }
    if result.len() > 6000 {
        result.truncate(6000);
        result.push_str("...(truncated)");
    }
    result.trim_end().to_string()
}

fn write_file(args: &str) -> String {
    let v = parse_args(args);
    let path_str = v["path"].as_str().unwrap_or("");
    let content = v["content"].as_str().unwrap_or("");
    if path_str.is_empty() {
        return "tool error: no path provided".to_string();
    }
    let path = safe_path(path_str);
    if !is_in_sandbox(&path) {
        return format!("tool error: writing outside sandbox is forbidden: {path}");
    }
    if let Some(parent) = std::path::Path::new(&path).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return format!("tool error: cannot create parent directories: {e}");
        }
    }
    match std::fs::write(&path, content) {
        Ok(_) => format!("ok: wrote {} bytes to {path}", content.len()),
        Err(e) => format!("tool error: {e}"),
    }
}

fn edit_file(args: &str) -> String {
    let v = parse_args(args);
    let path_str = v["path"].as_str().unwrap_or("");
    let old_string = v["old_string"].as_str().unwrap_or("");
    let new_string = v["new_string"].as_str().unwrap_or("");
    if path_str.is_empty() {
        return "tool error: no path provided".to_string();
    }
    if old_string.is_empty() {
        return "tool error: old_string cannot be empty".to_string();
    }
    let path = safe_path(path_str);
    if !is_in_sandbox(&path) {
        return format!("tool error: editing outside sandbox is forbidden: {path}");
    }
    let original = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => return format!("tool error: cannot read {path}: {e}"),
    };
    let count = original.matches(old_string).count();
    if count == 0 {
        return format!("tool error: old_string not found in {path}");
    }
    let modified = original.replace(old_string, new_string);
    match std::fs::write(&path, &modified) {
        Ok(_) => format!("ok: {count} replacement(s) made in {path}"),
        Err(e) => format!("tool error: {e}"),
    }
}

async fn web_search(args: &str) -> String {
    let v = parse_args(args);
    let query = v["query"].as_str().unwrap_or("");
    if query.is_empty() {
        return "tool error: no query provided".to_string();
    }
    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencode(query));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap();
    let resp = match client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return format!("tool error: search request failed: {e}"),
    };
    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => return format!("tool error: cannot read response: {e}"),
    };
    let mut results = Vec::new();
    for line in body.lines() {
        if line.contains("result__a") && line.contains("result__snippet") {
            results.push(line.to_string());
        }
    }
    if results.is_empty() {
        for line in body.lines() {
            if line.contains("class=\"result__a\"") || line.contains("class=\"result__snippet\"") {
                results.push(line.to_string());
            }
        }
    }
    if results.is_empty() {
        return format!("no results found for: {query}");
    }
    let mut output = String::new();
    let mut title = String::new();
    let mut snippet = String::new();
    let mut count = 0;
    for line in &results {
        if count >= 8 {
            break;
        }
        if line.contains("result__a") {
            if !title.is_empty() || !snippet.is_empty() {
                output.push_str(&format!(
                    "{}. {}\n   {}\n\n",
                    count + 1,
                    strip_html(&title),
                    strip_html(&snippet)
                ));
                count += 1;
                title.clear();
                snippet.clear();
            }
            title = extract_text(line);
        }
        if line.contains("result__snippet") {
            snippet = extract_text(line);
        }
    }
    if count < 8 && (!title.is_empty() || !snippet.is_empty()) {
        output.push_str(&format!(
            "{}. {}\n   {}\n",
            count + 1,
            strip_html(&title),
            strip_html(&snippet)
        ));
    }
    if output.is_empty() {
        return format!("no parseable results for: {query}");
    }
    output.trim_end().to_string()
}

fn extract_text(line: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in line.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
}

fn strip_html(s: &str) -> String {
    let out = s
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&#x27;", "'");
    let parts: Vec<&str> = out.split_whitespace().collect();
    parts.join(" ")
}

async fn web_fetch(args: &str) -> String {
    let v = parse_args(args);
    let url_str = v["url"].as_str().unwrap_or("");
    if url_str.is_empty() {
        return "tool error: no url provided".to_string();
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap();
    let resp = match client
        .get(url_str)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return format!("tool error: fetch failed: {e}"),
    };
    let status = resp.status();
    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => return format!("tool error: cannot read body: {e}"),
    };
    if !status.is_success() {
        return format!("tool error: HTTP {status}");
    }
    if body.len() > 8000 {
        let truncated: String = body.chars().take(8000).collect();
        format!("{truncated}\n...(truncated, {} total chars)", body.len())
    } else {
        body
    }
}

fn list_directory(args: &str) -> String {
    let v = parse_args(args);
    let sb = sandbox();
    let path_str = v["path"].as_str().unwrap_or(&sb);
    let path = safe_path(path_str);
    let dir = match std::fs::read_dir(&path) {
        Ok(d) => d,
        Err(e) => return format!("tool error: cannot read directory {path}: {e}"),
    };
    let mut entries: Vec<String> = Vec::new();
    for entry in dir {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = match entry.file_type() {
            Ok(ft) => {
                if ft.is_dir() {
                    "/"
                } else if ft.is_symlink() {
                    "@"
                } else {
                    ""
                }
            }
            Err(_) => "",
        };
        let size = match entry.metadata() {
            Ok(m) if m.is_file() => {
                let s = m.len();
                if s < 1024 {
                    format!(" ({s}B)")
                } else if s < 1024 * 1024 {
                    format!(" ({}KB)", s / 1024)
                } else {
                    format!(" ({}MB)", s / 1024 / 1024)
                }
            }
            _ => String::new(),
        };
        entries.push(format!("  {name}{file_type}{size}"));
    }
    entries.sort_by(|a, b| {
        let a_dir = a.contains('/');
        let b_dir = b.contains('/');
        if a_dir != b_dir {
            b_dir.cmp(&a_dir)
        } else {
            a.to_lowercase().cmp(&b.to_lowercase())
        }
    });
    if entries.is_empty() {
        format!("(empty directory: {path})")
    } else {
        let count = entries.len();
        if entries.len() > 200 {
            entries.truncate(200);
            entries.push(format!("  ... and {} more entries", count - 200));
        }
        format!("{path}:\n{}", entries.join("\n"))
    }
}

fn search_files(args: &str) -> String {
    let v = parse_args(args);
    let pattern = v["pattern"].as_str().unwrap_or("");
    if pattern.is_empty() {
        return "tool error: no pattern provided".to_string();
    }
    let sb = sandbox();
    let dir = v["directory"].as_str().unwrap_or(&sb);
    let result = Command::new("fd")
        .args(["--hidden", "--no-ignore", pattern, dir])
        .output();
    match result {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() {
                return format!("no files matching '{pattern}' in {dir}");
            }
            let lines: Vec<&str> = stdout.trim().lines().collect();
            let total = lines.len();
            if total > 200 {
                let truncated: Vec<&str> = lines.into_iter().take(200).collect();
                format!("{}\n...(and {} more)", truncated.join("\n"), total.saturating_sub(200))
            } else {
                stdout.trim().to_string()
            }
        }
        _ => {
            let result = Command::new("find")
                .args([dir, "-name", pattern])
                .output();
            match result {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    if stdout.trim().is_empty() {
                        format!("no files matching '{pattern}' in {dir}")
                    } else {
                        stdout.trim().to_string()
                    }
                }
                Ok(_) => format!("tool error: find failed"),
                Err(e) => format!("tool error: {e}"),
            }
        }
    }
}

fn search_content(args: &str) -> String {
    let v = parse_args(args);
    let pattern = v["pattern"].as_str().unwrap_or("");
    if pattern.is_empty() {
        return "tool error: no pattern provided".to_string();
    }
    let sb = sandbox();
    let dir = v["directory"].as_str().unwrap_or(&sb);
    let result = Command::new("rg")
        .args([
            "--no-heading",
            "--line-number",
            "--smart-case",
            "--hidden",
            "--no-ignore",
            pattern,
            dir,
        ])
        .output();
    match result {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() {
                return format!("no matches for '{pattern}' in {dir}");
            }
            let lines: Vec<&str> = stdout.trim().lines().collect();
            let total = lines.len();
            if total > 200 {
                let truncated: Vec<&str> = lines.into_iter().take(200).collect();
                format!("{}\n...(and {} more)", truncated.join("\n"), total.saturating_sub(200))
            } else {
                stdout.trim().to_string()
            }
        }
        _ => {
            let result = Command::new("grep")
                .args(["-rn", "--", pattern, dir])
                .output();
            match result {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    if stdout.trim().is_empty() {
                        format!("no matches for '{pattern}' in {dir}")
                    } else {
                        stdout.trim().to_string()
                    }
                }
                Ok(_) => format!("tool error: grep failed"),
                Err(e) => format!("tool error: {e}"),
            }
        }
    }
}

fn system_info() -> String {
    let mut info = String::new();
    if let Ok(out) = Command::new("hostname").output() {
        let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
        info.push_str(&format!("Hostname: {name}\n"));
    }
    if let Ok(out) = Command::new("sw_vers").output() {
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            let trimmed = line.trim();
            let key = trimmed
                .replace("ProductName:", "OS:")
                .replace("ProductVersion:", "Version:")
                .replace("BuildVersion:", "Build:");
            info.push_str(&format!("{key}\n"));
        }
    }
    if let Ok(out) = Command::new("uname").arg("-m").output() {
        let arch = String::from_utf8_lossy(&out.stdout).trim().to_string();
        info.push_str(&format!("Architecture: {arch}\n"));
    }
    if let Ok(out) = Command::new("uptime").output() {
        let up = String::from_utf8_lossy(&out.stdout).trim().to_string();
        info.push_str(&format!("Uptime: {up}\n"));
    }
    if let Ok(out) = Command::new("df").args(["-h", &sandbox()]).output() {
        let disk = String::from_utf8_lossy(&out.stdout);
        for line in disk.lines().skip(1) {
            info.push_str(&format!("Disk: {line}\n"));
        }
    }
    if info.is_empty() {
        "no system info available".to_string()
    } else {
        info.trim_end().to_string()
    }
}

async fn weather(args: &str) -> String {
    let v = parse_args(args);
    let city = v["city"].as_str().unwrap_or("Beijing");
    if city.is_empty() {
        return "tool error: no city provided".to_string();
    }
    let url = format!("https://wttr.in/{}?format=3", urlencode(city));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();
    match client
        .get(&url)
        .header("User-Agent", "curl/8.0")
        .send()
        .await
    {
        Ok(resp) => match resp.text().await {
            Ok(body) => body.trim().to_string(),
            Err(e) => format!("tool error: {e}"),
        },
        Err(e) => format!("tool error: weather request failed: {e}"),
    }
}

fn calculator(args: &str) -> String {
    let v = parse_args(args);
    let expr = v["expression"].as_str().unwrap_or("");
    if expr.is_empty() {
        return "tool error: no expression provided".to_string();
    }
    let safe = expr
        .chars()
        .all(|c| {
            c.is_ascii_digit()
                || c == '+'
                || c == '-'
                || c == '*'
                || c == '/'
                || c == '.'
                || c == '('
                || c == ')'
                || c == ' '
                || c == '%'
                || c == 'e'
                || c == 'E'
        });
    if !safe {
        return "tool error: expression contains unsafe characters".to_string();
    }
    let python_code = format!(
        "import math; r={}; print(int(r) if isinstance(r,float) and r.is_integer() else r)",
        expr
    );
    match Command::new("python3")
        .args(["-c", &python_code])
        .output()
    {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !stderr.is_empty() {
                format!("tool error: {stderr}", stderr = stderr.trim())
            } else {
                stdout.trim().to_string()
            }
        }
        Err(e) => format!("tool error: {e}"),
    }
}

fn brew_info(args: &str) -> String {
    let v = parse_args(args);
    let pkg = v["package"].as_str().unwrap_or("");
    if pkg.is_empty() {
        return "tool error: no package name provided".to_string();
    }
    match Command::new("brew")
        .args(["info", "--json=v2", pkg])
        .output()
    {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                return format!("tool error: brew failed: {stderr}", stderr = stderr.trim());
            }
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() {
                return format!("no brew info for {pkg}");
            }
            match serde_json::from_str::<serde_json::Value>(&stdout) {
                Ok(json) => {
                    if let Some(formulae) = json.get("formulae").and_then(|f| f.as_array()) {
                        if let Some(f) = formulae.first() {
                            return format_brew_entry(f, pkg);
                        }
                    }
                    if let Some(casks) = json.get("casks").and_then(|c| c.as_array()) {
                        if let Some(c) = casks.first() {
                            return format_brew_entry(c, pkg);
                        }
                    }
                    format!("package {pkg} not found in brew")
                }
                Err(_) => stdout.trim().to_string(),
            }
        }
        Err(e) => format!("tool error: {e}"),
    }
}

fn format_brew_entry(entry: &serde_json::Value, name: &str) -> String {
    let full_name = entry["full_name"].as_str().unwrap_or(name);
    let version = entry
        .get("versions")
        .and_then(|v| v.get("stable"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let desc = entry["desc"].as_str().unwrap_or("");
    let homepage = entry["homepage"].as_str().unwrap_or("");
    let installed = entry
        .get("installed")
        .and_then(|i| i.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    let status = if installed { "installed" } else { "not installed" };
    format!(
        "{full_name} ({status})\nversion: {version}\ndesc: {desc}\nhomepage: {homepage}"
    )
}

fn todowrite(args: &str) -> String {
    let v = parse_args(args);
    if let Some(todos) = v["todos"].as_array() {
        let count = todos.len();
        let done = todos
            .iter()
            .filter(|t| t["status"].as_str() == Some("completed"))
            .count();
        format!("ok: {done}/{count} tasks completed")
    } else if let Ok(todos) = serde_json::from_str::<Vec<serde_json::Value>>(
        v["todos"].as_str().unwrap_or("[]"),
    ) {
        let count = todos.len();
        let done = todos
            .iter()
            .filter(|t| t["status"].as_str() == Some("completed"))
            .count();
        format!("ok: {done}/{count} tasks completed")
    } else {
        "ok: todo list updated".to_string()
    }
}

fn ask_question() -> String {
    "tool error: ask_question not available in non-interactive mode".to_string()
}
