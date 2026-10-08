//! 后端词条表，按域拆分为多个子模块；每个子模块的 `text` 未命中时返回 None。

use super::Lang;

pub mod agent;
pub mod cmd_core;
pub mod cmd_provider;
pub mod common;
pub mod proxy;
pub mod service_data;
pub mod service_runtime;

pub fn lookup(lang: Lang, key: &str) -> Option<&'static str> {
    common::text(lang, key)
        .or_else(|| agent::text(lang, key))
        .or_else(|| cmd_core::text(lang, key))
        .or_else(|| cmd_provider::text(lang, key))
        .or_else(|| service_data::text(lang, key))
        .or_else(|| service_runtime::text(lang, key))
        .or_else(|| proxy::text(lang, key))
}
