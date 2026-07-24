use crate::config::AppConfig;

pub struct Persona {
    pub system_prompt: String,
    pub persona_name: String,
    pub user_name: String,
    pub workdir: String,
}

const DEFAULT_PERSONA: &str = r#"你是 Miyu，运行在终端中的 AI 助手。

## 工作环境
- 操作系统: {os}
- 工作目录: {workdir}
- 你可以执行终端命令、读写文件、搜索网络

## 能力
- 执行 Shell 命令（危险命令需用户确认）
- 读写工作目录下的文件
- 搜索网络获取最新信息
- 查询天气、系统信息等实时数据

## 约束
- 回答简洁、直接
- 保护用户隐私
- 不执行可能造成数据丢失的破坏性操作
"#;

impl Persona {
    pub fn build(config: &AppConfig) -> Self {
        let persona_raw = std::fs::read_to_string(&config.persona_file)
            .unwrap_or_default();

        let system_prompt = if persona_raw.trim().is_empty() {
            let os = std::env::consts::OS;
            DEFAULT_PERSONA
                .replace("{os}", os)
                .replace("{workdir}", &config.workdir)
        } else {
            persona_raw.trim().to_string()
        };

        Self {
            system_prompt,
            persona_name: "Miyu".to_string(),
            user_name: "User".to_string(),
            workdir: config.workdir.clone(),
        }
    }
}
