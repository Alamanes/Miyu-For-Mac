use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub persona_file: String,
    pub workdir: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let workdir = std::env::var("MIYU_WORKDIR")
            .or_else(|_| std::env::current_dir().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_else(|_| ".".to_string());

        let persona_file = std::env::var("MIYU_PERSONA")
            .unwrap_or_else(|_| format!("{home}/.miyu/persona.md"));

        if let Some(parent) = std::path::Path::new(&persona_file).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        Ok(Self {
            persona_file,
            workdir,
        })
    }
}
