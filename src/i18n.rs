use std::sync::OnceLock;

static IS_ZH: OnceLock<bool> = OnceLock::new();

pub fn init() {
    let lang = std::env::var("LANG").unwrap_or_default();
    IS_ZH.set(lang.starts_with("zh")).ok();
}

pub fn is_zh() -> bool {
    *IS_ZH.get().unwrap_or(&false)
}

pub fn t(en: &'static str, zh: &'static str) -> &'static str {
    if is_zh() { zh } else { en }
}
