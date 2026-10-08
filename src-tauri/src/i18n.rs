//! 轻量后端国际化：语言枚举、系统语言探测、全局当前语言、文案表驱动翻译。
//!
//! 瞬时文案（报错、托盘、HTTP 响应体）在生成时按当前语言解析；
//! 需要跨语言切换重放的内容（系统日志、请求日志）不走这里，改为存 key + params。

use sqlx::SqlitePool;
use std::sync::atomic::{AtomicU8, Ordering};

mod text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Zh => "zh-CN",
            Lang::En => "en",
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            Lang::Zh => 0,
            Lang::En => 1,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            1 => Lang::En,
            _ => Lang::Zh,
        }
    }
}

static CURRENT_LANG: AtomicU8 = AtomicU8::new(0);

pub fn set_current_lang(lang: Lang) {
    CURRENT_LANG.store(lang.as_u8(), Ordering::Relaxed);
}

pub fn current_lang() -> Lang {
    Lang::from_u8(CURRENT_LANG.load(Ordering::Relaxed))
}

/// 探测系统语言：以 zh 开头 → 中文，其余一律 → 英文
pub fn detect_system_lang() -> Lang {
    match sys_locale::get_locale() {
        Some(locale) if locale.to_ascii_lowercase().starts_with("zh") => Lang::Zh,
        _ => Lang::En,
    }
}

/// 解析语言设置值（auto / zh-CN / en）
pub fn resolve_lang(setting: &str) -> Lang {
    match setting.trim() {
        "zh-CN" => Lang::Zh,
        "en" => Lang::En,
        _ => detect_system_lang(),
    }
}

/// 读取数据库中的语言设置（gateway_settings.language）
pub async fn language_setting(db: &SqlitePool) -> String {
    sqlx::query_as::<_, (String,)>("SELECT language FROM gateway_settings WHERE id = 1")
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .map(|row| row.0)
        .unwrap_or_else(|| "auto".to_string())
}

/// 按数据库设置初始化全局语言，返回解析结果
pub async fn init_from_db(db: &SqlitePool) -> Lang {
    let lang = resolve_lang(&language_setting(db).await);
    set_current_lang(lang);
    lang
}

/// 文案表：key → 各语言模板（`{}` 为按顺序替换的占位符）。
///
/// 词条按域拆分在 `i18n/text/` 子模块中，未收录的 key 返回 None，
/// 由调用方原样透出 key，便于在界面/日志中直接发现遗漏。
pub fn key_text(lang: Lang, key: &str) -> Option<&'static str> {
    text::lookup(lang, key)
}

pub fn t_with(lang: Lang, key: &str, args: &[String]) -> String {
    render(key_text(lang, key).unwrap_or(key), args)
}

/// 按全局当前语言翻译
pub fn t(key: &str, args: &[String]) -> String {
    t_with(current_lang(), key, args)
}

fn render(template: &str, args: &[String]) -> String {
    if args.is_empty() || !template.contains("{}") {
        return template.to_string();
    }

    let mut output = String::with_capacity(template.len() + 32);
    let mut rest = template;
    let mut index = 0;
    while let Some(position) = rest.find("{}") {
        output.push_str(&rest[..position]);
        match args.get(index) {
            Some(arg) => output.push_str(arg),
            None => output.push_str("{}"),
        }
        rest = &rest[position + 2..];
        index += 1;
    }
    output.push_str(rest);
    output
}

/// 用法：`tr!("tray.show")` / `tr!("err.x", value)`
///
/// 参数在宏内先转成 `String`（`dyn Display` 不是 `Send`，跨越 await 会破坏 future 的 Send）
#[macro_export]
macro_rules! tr {
    ($key:expr) => {
        $crate::i18n::t($key, &[])
    };
    ($key:expr, $($arg:expr),+ $(,)?) => {
        $crate::i18n::t($key, &[$($arg.to_string()),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_falls_back_to_detection_and_never_panics() {
        assert_eq!(resolve_lang("zh-CN"), Lang::Zh);
        assert_eq!(resolve_lang("en"), Lang::En);
        assert_eq!(resolve_lang("auto"), detect_system_lang());
        assert_eq!(resolve_lang("unknown"), detect_system_lang());
    }

    #[test]
    fn render_handles_missing_and_extra_args() {
        assert_eq!(render("a {} b", &[]), "a {} b");
        assert_eq!(render("a {} {}", &["1".to_string(), "2".to_string()]), "a 1 2");
        assert_eq!(render("a {} {}", &["1".to_string()]), "a 1 {}");
        assert_eq!(render("no placeholders", &["1".to_string()]), "no placeholders");
        assert_eq!(
            render("x {} y {}", &["A".to_string(), "B".to_string()]),
            "x A y B"
        );
    }
}
