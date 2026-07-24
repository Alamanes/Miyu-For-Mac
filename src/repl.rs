use anyhow::Result;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use crate::llm::{ChatMessage, OpencodeClient, ToolDefinition};
use crate::persona::Persona;
use crate::render::StreamRenderer;
use crate::tools::ToolRegistry;

pub async fn run(persona: &Persona, client: OpencodeClient, tools: ToolRegistry, _show_reasoning: bool, _tools_enabled: bool) -> Result<()> {
    let renderer = Arc::new(Mutex::new(StreamRenderer::new()));
    let mut history: Vec<ChatMessage> = vec![ChatMessage::system(&persona.system_prompt)];
    let mut mode = String::from("normal");

    println!("\n  ✨ Miyu — {} ✨", persona.persona_name);
    println!("  Terminal AI Assistant\n");
    println!("  /help 查看命令, /exit 退出\n");

    let stdin = io::stdin();
    loop {
        let mut input = String::new();
        print!("> ");
        io::stdout().flush()?;
        if stdin.read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        if input.starts_with('/') {
            match handle_command(&input, &mut history, &renderer, &mut mode) {
                CommandAction::Exit => break,
                CommandAction::Continue => continue,
            }
        }

        history.push(ChatMessage::user(&input));
        chat_loop(&client, &tools, &renderer, &mut history, &mode).await?;
    }

    println!("\n  Goodbye!\n");
    Ok(())
}

pub async fn run_once(
    persona: &Persona,
    client: OpencodeClient,
    tools: ToolRegistry,
    message: &str,
    _show_reasoning: bool,
    _tools_enabled: bool,
) -> Result<()> {
    let renderer = Arc::new(Mutex::new(StreamRenderer::new()));
    let mut history = vec![
        ChatMessage::system(&persona.system_prompt),
        ChatMessage::user(message),
    ];
    let mode = String::from("normal");
    chat_loop(&client, &tools, &renderer, &mut history, &mode).await?;
    Ok(())
}

enum CommandAction {
    Exit,
    Continue,
}

fn handle_command(
    cmd: &str,
    history: &mut Vec<ChatMessage>,
    renderer: &Arc<Mutex<StreamRenderer>>,
    mode: &mut String,
) -> CommandAction {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    match parts[0] {
        "/exit" | "/quit" => {
            renderer.lock().unwrap().hide_spinner();
            return CommandAction::Exit;
        }
        "/help" => {
            println!(
                "Commands:\n  /help    Show help\n  /clear   Clear conversation history\n  /mode    normal|chat|plan\n  /exit    Quit"
            );
        }
        "/clear" => {
            history.retain(|m| m.role == "system");
            println!("  Cleared conversation history.");
        }
        "/mode" => {
            if let Some(m) = parts.get(1) {
                match *m {
                    "normal" | "chat" | "plan" => {
                        *mode = m.to_string();
                        println!("  Mode: {mode}");
                    }
                    _ => println!("  Invalid mode. Use: normal, chat, plan"),
                }
            } else {
                println!("  Current mode: {mode}");
            }
        }
        _ => println!("  Unknown command: {}", parts[0]),
    }
    CommandAction::Continue
}

async fn chat_loop(
    client: &OpencodeClient,
    tools: &ToolRegistry,
    renderer: &Arc<Mutex<StreamRenderer>>,
    history: &mut Vec<ChatMessage>,
    mode: &str,
) -> Result<()> {
    let tool_defs: Vec<ToolDefinition> = if mode == "chat" {
        vec![]
    } else {
        tools.definitions()
    };
    let mut session_tokens: u64 = 0;

    for _round in 0..10 {
        renderer.lock().unwrap().show_spinner("thinking...");

        let r = renderer.clone();
        let result = client
            .chat_stream(history, &tool_defs, move |chunk| {
                if let Ok(mut renderer) = r.lock() {
                    let _ = renderer.write_chunk(&chunk);
                }
            })
            .await?;

        renderer.lock().unwrap().hide_spinner();
        renderer.lock().unwrap().finish()?;

        if let Some(ref usage) = result.usage {
            let turn = usage.total_tokens;
            session_tokens += turn;
            renderer
                .lock()
                .unwrap()
                .print_token_usage(turn, session_tokens)?;
        }

        if result.tool_calls.is_empty() {
            if !result.content.is_empty() {
                history.push(ChatMessage::assistant(&result.content, None));
            }
            break;
        }

        if !result.content.is_empty() {
            history.push(ChatMessage::assistant(&result.content, Some(result.tool_calls.clone())));
        } else {
            history.push(ChatMessage::assistant("", Some(result.tool_calls.clone())));
        }

        for tc in &result.tool_calls {
            {
                let mut r = renderer.lock().unwrap();
                let _ = r.write_tool_call(&tc.function.name);
            }

            let tool_result = tools.call(&tc.function.name, &tc.function.arguments).await;

            let is_error = tool_result.starts_with("tool error:");
            let ok = !is_error;
            let brief = if tool_result.len() > 120 {
                let snipped: String = tool_result
                    .chars()
                    .take(117)
                    .collect();
                format!("{snipped}...")
            } else {
                tool_result.clone()
            };
            {
                let mut r = renderer.lock().unwrap();
                let _ = r.write_tool_result(ok, &brief);
            }

            history.push(ChatMessage::tool(&tc.id, &tool_result));
        }
    }

    Ok(())
}
