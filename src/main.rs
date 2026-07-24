mod config;
mod config_tui;
mod i18n;
mod llm;
mod persona;
mod render;
mod repl;
mod session;
mod tools;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "miyu", version, about = "Miyu — terminal AI assistant")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(long, hide = true)]
    shell_intercept: bool,

    #[arg(long, hide = true)]
    shell: Option<String>,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    message: Vec<String>,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Open configuration TUI")]
    Config,
    #[command(about = "Show configuration paths")]
    Paths,
    #[command(about = "Install fish shell integration")]
    FishInit,
    #[command(about = "Install bash shell integration")]
    BashInit,
    #[command(about = "Install zsh shell integration")]
    ZshInit,
    #[command(about = "Show current session info")]
    Session,
}

#[tokio::main]
async fn main() -> Result<()> {
    i18n::init();
    let cli = Cli::parse();

    if cli.shell_intercept {
        return run_shell_intercept(&cli).await;
    }

    match cli.command {
        Some(Command::Config) => {
            config_tui::run().await?;
            return Ok(());
        }
        Some(Command::Paths) => {
            let app_cfg = config::AppConfig::load()?;
            let local = config_tui::MiyuLocalConfig::load();
            let home = std::env::var("HOME").unwrap_or_default();
            println!("Persona file:     {}", app_cfg.persona_file);
            println!("Work directory:   {}", app_cfg.workdir);
            println!("Config file:      {home}/.miyu_macos.json");
            println!("Sessions dir:     {home}/.miyu/sessions/");
            println!("LLM endpoint:     {}", local.active_url());
            println!("LLM model:        {}", local.active_model_name());
            println!("Providers:        {}", local.providers.len());
            return Ok(());
        }
        Some(Command::FishInit) => {
            install_fish_hook()?;
            return Ok(());
        }
        Some(Command::BashInit) => {
            install_bash_hook()?;
            return Ok(());
        }
        Some(Command::ZshInit) => {
            install_zsh_hook()?;
            return Ok(());
        }
        Some(Command::Session) => {
            let sessions = session::list_sessions();
            let current = session::current_session_id();
            println!("Current session: {current}");
            println!("All sessions:");
            for s in &sessions {
                let marker = if s == &current { " *" } else { "  " };
                println!("{marker}{s}");
            }
            return Ok(());
        }
        None => {}
    }

    let app_cfg = config::AppConfig::load()?;
    let persona = persona::Persona::build(&app_cfg);
    let local = config_tui::MiyuLocalConfig::load();
    let client = llm::OpencodeClient::new(
        Some(&local.active_url()),
        Some(&local.active_model_name()),
    );
    let tool_registry = tools::ToolRegistry::new();

    let message = cli.message.join(" ");
    if message.trim().is_empty() {
        repl::run(&persona, client, tool_registry, local.show_reasoning, local.tools_enabled).await?;
    } else {
        repl::run_once(&persona, client, tool_registry, &message, local.show_reasoning, local.tools_enabled).await?;
    }

    Ok(())
}

async fn run_shell_intercept(cli: &Cli) -> Result<()> {
    let message = cli.message.join(" ");
    if message.trim().is_empty() {
        return Ok(());
    }

    let app_cfg = config::AppConfig::load()?;
    let persona = persona::Persona::build(&app_cfg);
    let local = config_tui::MiyuLocalConfig::load();

    let mut sess = session::Session::load_or_create(&persona.persona_name);

    if sess.messages.is_empty() {
        sess.messages.push(llm::ChatMessage::system(&persona.system_prompt));
    }

    sess.add_user(&message);

    let client = llm::OpencodeClient::new(
        Some(&local.active_url()),
        Some(&local.active_model_name()),
    );

    let mut response_text = String::new();
    let renderer = std::sync::Arc::new(std::sync::Mutex::new(
        render::StreamRenderer::new_with_reasoning(local.show_reasoning),
    ));

    {
        let r = renderer.lock().unwrap();
        r.show_spinner("thinking...");
    }

    let r = renderer.clone();
    let result = client
        .chat_stream(&sess.messages, &[], move |chunk| {
            if let Ok(mut renderer) = r.lock() {
                let _ = renderer.write_chunk(&chunk);
            }
        })
        .await?;

    if let Ok(mut r) = renderer.lock() {
        let _ = r.finish();
    }

    response_text = result.content.clone();

    if !response_text.trim().is_empty() {
        sess.add_assistant(&response_text);
    }
    sess.save()?;

    Ok(())
}

fn install_fish_hook() -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let func_dir = format!("{home}/.config/fish/functions");
    std::fs::create_dir_all(&func_dir)?;

    let func_path = format!("{func_dir}/fish_command_not_found.fish");
    let content = r#"function fish_command_not_found
    miyu --shell-intercept "$argv"
end
"#;

    std::fs::write(&func_path, content)?;
    println!("Fish integration installed.");
    println!("Restart your shell or run: source ~/.config/fish/config.fish");
    Ok(())
}

fn install_bash_hook() -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let bashrc = format!("{home}/.bashrc");
    let hook = r#"
command_not_found_handle() {
    miyu --shell-intercept "$*" 2>/dev/null
}
"#;

    let existing = std::fs::read_to_string(&bashrc).unwrap_or_default();
    if !existing.contains("miyu --shell-intercept") {
        std::fs::write(&bashrc, format!("{existing}\n{hook}\n"))?;
        println!("Bash integration installed. Run: source ~/.bashrc");
    } else {
        println!("Bash integration already installed.");
    }
    Ok(())
}

fn install_zsh_hook() -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let zshrc = format!("{home}/.zshrc");
    let hook = r#"
command_not_found_handler() {
    miyu --shell-intercept "$*" 2>/dev/null
}
"#;

    let existing = std::fs::read_to_string(&zshrc).unwrap_or_default();
    if !existing.contains("miyu --shell-intercept") {
        std::fs::write(&zshrc, format!("{existing}\n{hook}\n"))?;
        println!("Zsh integration installed. Run: source ~/.zshrc");
    } else {
        println!("Zsh integration already installed.");
    }
    Ok(())
}
