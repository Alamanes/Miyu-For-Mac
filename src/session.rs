use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::llm::ChatMessage;

fn sessions_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".miyu").join("sessions")
}

pub fn current_session_id() -> String {
    std::env::var("MIYU_SESSION")
        .or_else(|_| {
            std::env::var("TTY").map(|tty| {
                std::path::Path::new(&tty)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "default".into())
                    .replace('/', "_")
            })
        })
        .unwrap_or_else(|_| "default".into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub persona_name: String,
}

impl Session {
    pub fn load_or_create(persona_name: &str) -> Self {
        let id = current_session_id();
        let path = sessions_dir().join(format!("{id}.json"));

        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(session) = serde_json::from_str::<Session>(&data) {
                return session;
            }
        }

        Self {
            id,
            messages: Vec::new(),
            persona_name: persona_name.to_string(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let dir = sessions_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", self.id));
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, format!("{json}\n"))?;
        Ok(())
    }

    pub fn add_user(&mut self, content: &str) {
        self.messages.push(ChatMessage::user(content));
    }

    pub fn add_assistant(&mut self, content: &str) {
        self.messages.push(ChatMessage::assistant(content, None));
    }
}

pub fn list_sessions() -> Vec<String> {
    let dir = sessions_dir();
    if !dir.exists() {
        return Vec::new();
    }
    let mut sessions = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".json") {
                sessions.push(name.trim_end_matches(".json").to_string());
            }
        }
    }
    sessions.sort();
    sessions
}
