"""CLI 配置同步服务 - 将 MCP 和 Prompt 配置同步到各 CLI 的配置文件"""
import json
from pathlib import Path
import logging

logger = logging.getLogger(__name__)


def get_home_dir() -> Path:
    """获取用户主目录"""
    return Path.home()


# ============================================================================
# MCP 配置同步
# ============================================================================

def get_claude_mcp_path() -> Path:
    """Claude Code MCP 配置路径: ~/.claude.json"""
    return get_home_dir() / ".claude.json"


def get_codex_config_path() -> Path:
    """Codex 配置路径: ~/.codex/config.toml"""
    return get_home_dir() / ".codex" / "config.toml"


def get_gemini_settings_path() -> Path:
    """Gemini 配置路径: ~/.gemini/settings.json"""
    return get_home_dir() / ".gemini" / "settings.json"


def sync_mcp_to_claude(mcp_name: str, config_json: str, enabled: bool) -> bool:
    """同步 MCP 到 Claude Code 配置"""
    mcp_path = get_claude_mcp_path()

    # 如果目录不存在，跳过（用户未安装 Claude Code）
    if not mcp_path.parent.exists() and not mcp_path.exists():
        logger.debug("Claude Code 未安装，跳过 MCP 同步")
        return True

    try:
        # 读取现有配置
        if mcp_path.exists():
            with open(mcp_path, 'r', encoding='utf-8') as f:
                data = json.load(f)
        else:
            data = {}

        # 确保 mcpServers 字段存在
        if "mcpServers" not in data:
            data["mcpServers"] = {}

        # 解析 MCP 配置
        mcp_config = json.loads(config_json)

        if enabled:
            data["mcpServers"][mcp_name] = mcp_config
        else:
            data["mcpServers"].pop(mcp_name, None)

        # 写回配置
        with open(mcp_path, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

        logger.info(f"已同步 MCP '{mcp_name}' 到 Claude Code (enabled={enabled})")
        return True
    except Exception as e:
        logger.error(f"同步 MCP 到 Claude Code 失败: {e}")
        return False


def sync_mcp_to_codex(mcp_name: str, config_json: str, enabled: bool) -> bool:
    """同步 MCP 到 Codex 配置"""
    config_path = get_codex_config_path()

    # 如果目录不存在，跳过
    if not config_path.parent.exists():
        logger.debug("Codex 未安装，跳过 MCP 同步")
        return True

    try:
        import tomli
        import tomli_w
    except ImportError:
        logger.warning("tomli/tomli_w 未安装，跳过 Codex MCP 同步")
        return False

    try:
        # 读取现有配置
        if config_path.exists():
            with open(config_path, 'rb') as f:
                data = tomli.load(f)
        else:
            data = {}

        # 确保 mcp_servers 字段存在
        if "mcp_servers" not in data:
            data["mcp_servers"] = {}

        # 解析 MCP 配置并转换为 TOML 格式
        mcp_config = json.loads(config_json)

        if enabled:
            # 转换 JSON 配置为 Codex TOML 格式（Codex 不使用 type 字段）
            toml_config = {}
            mcp_type = mcp_config.get("type", "stdio")

            if mcp_type == "stdio":
                # STDIO 服务器：command, args, env, env_vars, cwd
                if "command" in mcp_config:
                    toml_config["command"] = mcp_config["command"]
                if mcp_config.get("args"):
                    toml_config["args"] = mcp_config["args"]
                if mcp_config.get("env"):
                    toml_config["env"] = mcp_config["env"]
                if mcp_config.get("env_vars"):
                    toml_config["env_vars"] = mcp_config["env_vars"]
                if mcp_config.get("cwd"):
                    toml_config["cwd"] = mcp_config["cwd"]
            else:
                # HTTP/SSE 服务器：url, bearer_token_env_var, http_headers, env_http_headers
                if "url" in mcp_config:
                    toml_config["url"] = mcp_config["url"]
                if mcp_config.get("bearer_token_env_var"):
                    toml_config["bearer_token_env_var"] = mcp_config["bearer_token_env_var"]
                if mcp_config.get("headers"):
                    toml_config["http_headers"] = mcp_config["headers"]
                if mcp_config.get("http_headers"):
                    toml_config["http_headers"] = mcp_config["http_headers"]
                if mcp_config.get("env_http_headers"):
                    toml_config["env_http_headers"] = mcp_config["env_http_headers"]

            # 通用可选字段
            if mcp_config.get("startup_timeout_sec"):
                toml_config["startup_timeout_sec"] = mcp_config["startup_timeout_sec"]
            if mcp_config.get("tool_timeout_sec"):
                toml_config["tool_timeout_sec"] = mcp_config["tool_timeout_sec"]
            if mcp_config.get("enabled_tools"):
                toml_config["enabled_tools"] = mcp_config["enabled_tools"]
            if mcp_config.get("disabled_tools"):
                toml_config["disabled_tools"] = mcp_config["disabled_tools"]

            data["mcp_servers"][mcp_name] = toml_config
        else:
            data["mcp_servers"].pop(mcp_name, None)

        # 写回配置
        with open(config_path, 'wb') as f:
            tomli_w.dump(data, f)

        logger.info(f"已同步 MCP '{mcp_name}' 到 Codex (enabled={enabled})")
        return True
    except Exception as e:
        logger.error(f"同步 MCP 到 Codex 失败: {e}")
        return False


def sync_mcp_to_gemini(mcp_name: str, config_json: str, enabled: bool) -> bool:
    """同步 MCP 到 Gemini 配置"""
    settings_path = get_gemini_settings_path()

    # 如果目录不存在，跳过
    if not settings_path.parent.exists():
        logger.debug("Gemini CLI 未安装，跳过 MCP 同步")
        return True

    try:
        # 读取现有配置
        if settings_path.exists():
            with open(settings_path, 'r', encoding='utf-8') as f:
                data = json.load(f)
        else:
            data = {}

        # 确保 mcpServers 字段存在
        if "mcpServers" not in data:
            data["mcpServers"] = {}

        # 解析 MCP 配置
        mcp_config = json.loads(config_json)

        if enabled:
            data["mcpServers"][mcp_name] = mcp_config
        else:
            data["mcpServers"].pop(mcp_name, None)

        # 写回配置
        with open(settings_path, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

        logger.info(f"已同步 MCP '{mcp_name}' 到 Gemini (enabled={enabled})")
        return True
    except Exception as e:
        logger.error(f"同步 MCP 到 Gemini 失败: {e}")
        return False


def sync_mcp_to_cli(mcp_name: str, config_json: str, cli_flags: dict, old_cli_flags: dict = None) -> dict:
    """
    同步 MCP 到发生变化的 CLI

    Args:
        mcp_name: MCP 名称
        config_json: MCP 配置 JSON
        cli_flags: 新的 CLI 开关状态
        old_cli_flags: 旧的 CLI 开关状态（用于判断哪些发生了变化）
    """
    results = {}

    # 如果没有旧状态，说明是新建，同步所有启用的 CLI
    if old_cli_flags is None:
        old_cli_flags = {"claude_code": False, "codex": False, "gemini": False}

    # 只同步发生变化的 CLI
    new_claude = cli_flags.get("claude_code", False)
    old_claude = old_cli_flags.get("claude_code", False)
    if new_claude != old_claude:
        results["claude_code"] = sync_mcp_to_claude(mcp_name, config_json, new_claude)

    new_codex = cli_flags.get("codex", False)
    old_codex = old_cli_flags.get("codex", False)
    if new_codex != old_codex:
        results["codex"] = sync_mcp_to_codex(mcp_name, config_json, new_codex)

    new_gemini = cli_flags.get("gemini", False)
    old_gemini = old_cli_flags.get("gemini", False)
    if new_gemini != old_gemini:
        results["gemini"] = sync_mcp_to_gemini(mcp_name, config_json, new_gemini)

    return results


def remove_mcp_from_all_cli(mcp_name: str) -> dict:
    """从所有 CLI 中移除 MCP"""
    empty_config = "{}"
    return {
        "claude_code": sync_mcp_to_claude(mcp_name, empty_config, False),
        "codex": sync_mcp_to_codex(mcp_name, empty_config, False),
        "gemini": sync_mcp_to_gemini(mcp_name, empty_config, False),
    }


# ============================================================================
# Prompt 配置同步
# ============================================================================

def get_claude_prompt_path() -> Path:
    """Claude Code 提示词路径: ~/.claude/CLAUDE.md"""
    return get_home_dir() / ".claude" / "CLAUDE.md"


def get_codex_prompt_path() -> Path:
    """Codex 提示词路径: ~/.codex/AGENTS.md"""
    return get_home_dir() / ".codex" / "AGENTS.md"


def get_gemini_prompt_path() -> Path:
    """Gemini 提示词路径: ~/.gemini/GEMINI.md"""
    return get_home_dir() / ".gemini" / "GEMINI.md"


def sync_prompt_to_file(file_path: Path, content: str, enabled: bool) -> bool:
    """同步提示词到指定文件"""
    # 如果目录不存在，跳过
    if not file_path.parent.exists():
        logger.debug(f"目录不存在，跳过提示词同步: {file_path.parent}")
        return True

    try:
        if enabled:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            logger.info(f"已同步提示词到 {file_path}")
        else:
            # 禁用时清空文件内容
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write("")
            logger.info(f"已清空提示词文件 {file_path}")
        return True
    except Exception as e:
        logger.error(f"同步提示词失败: {e}")
        return False


def sync_prompt_to_cli(prompt_name: str, content: str, cli_flags: dict, old_cli_flags: dict = None) -> dict:
    """
    同步提示词到发生变化的 CLI

    Args:
        prompt_name: 提示词名称
        content: 提示词内容
        cli_flags: 新的 CLI 开关状态
        old_cli_flags: 旧的 CLI 开关状态
    """
    results = {}

    if old_cli_flags is None:
        old_cli_flags = {"claude_code": False, "codex": False, "gemini": False}

    # 只同步发生变化的 CLI
    new_claude = cli_flags.get("claude_code", False)
    old_claude = old_cli_flags.get("claude_code", False)
    if new_claude != old_claude:
        results["claude_code"] = sync_prompt_to_file(get_claude_prompt_path(), content, new_claude)

    new_codex = cli_flags.get("codex", False)
    old_codex = old_cli_flags.get("codex", False)
    if new_codex != old_codex:
        results["codex"] = sync_prompt_to_file(get_codex_prompt_path(), content, new_codex)

    new_gemini = cli_flags.get("gemini", False)
    old_gemini = old_cli_flags.get("gemini", False)
    if new_gemini != old_gemini:
        results["gemini"] = sync_prompt_to_file(get_gemini_prompt_path(), content, new_gemini)

    return results

# ============================================================================
# CLI 启用配置同步（服务地址和 API KEY）
# ============================================================================

def get_claude_settings_path() -> Path:
    """Claude Code settings 路径: ~/.claude/settings.json"""
    return get_home_dir() / ".claude" / "settings.json"


def sync_claude_settings(base_url: str, api_key: str, default_json_config: str, enabled: bool) -> bool:
    """同步 Claude Code 配置"""
    settings_path = get_claude_settings_path()

    if not settings_path.parent.exists():
        logger.debug("Claude Code 未安装，跳过配置同步")
        return True

    try:
        # 读取现有配置，处理文件为空的情况
        data = {}
        if settings_path.exists():
            content = settings_path.read_text(encoding='utf-8').strip()
            if content:
                data = json.loads(content)

        if enabled:
            if "env" not in data:
                data["env"] = {}
            data["env"]["ANTHROPIC_BASE_URL"] = base_url
            data["env"]["ANTHROPIC_AUTH_TOKEN"] = api_key

            # 合并用户自定义配置（仅当配置非空时）
            if default_json_config and default_json_config.strip():
                try:
                    custom_config = json.loads(default_json_config)
                    _deep_merge(data, custom_config)
                except json.JSONDecodeError as e:
                    logger.warning(f"Claude Code 自定义配置解析失败（非有效 JSON）: {e}")
        else:
            # 禁用时移除配置
            if "env" in data:
                data["env"].pop("ANTHROPIC_BASE_URL", None)
                data["env"].pop("ANTHROPIC_AUTH_TOKEN", None)
            # 移除自定义配置中的字段
            if default_json_config and default_json_config.strip():
                try:
                    custom_config = json.loads(default_json_config)
                    _remove_keys(data, custom_config)
                except json.JSONDecodeError:
                    pass

        with open(settings_path, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

        logger.info(f"已同步 Claude Code 配置 (enabled={enabled})")
        return True
    except Exception as e:
        logger.error(f"同步 Claude Code 配置失败: {e}")
        return False


def get_codex_auth_path() -> Path:
    """Codex 认证文件路径: ~/.codex/auth.json"""
    return get_home_dir() / ".codex" / "auth.json"


CODEX_PROVIDER_KEY = "ccg-gateway"
CODEX_AUTH_KEY = "OPENAI_API_KEY"


def sync_codex_settings(base_url: str, api_key: str, default_toml_config: str, enabled: bool) -> bool:
    """同步 Codex 配置"""
    config_path = get_codex_config_path()
    auth_path = get_codex_auth_path()

    if not config_path.parent.exists():
        logger.debug("Codex 未安装，跳过配置同步")
        return True

    try:
        import tomli
        import tomli_w
    except ImportError:
        logger.warning("tomli/tomli_w 未安装，跳过 Codex 配置同步")
        return False

    try:
        if config_path.exists():
            with open(config_path, 'rb') as f:
                data = tomli.load(f)
        else:
            data = {}

        if enabled:
            # 设置活动 provider
            data["model_provider"] = CODEX_PROVIDER_KEY

            # 创建自定义 provider
            if "model_providers" not in data:
                data["model_providers"] = {}
            data["model_providers"][CODEX_PROVIDER_KEY] = {
                "name": CODEX_PROVIDER_KEY,
                "base_url": base_url,
                "wire_api": "responses",
                "requires_openai_auth": False,
            }

            # 写入 auth.json
            auth_data = {CODEX_AUTH_KEY: api_key}
            with open(auth_path, 'w', encoding='utf-8') as f:
                json.dump(auth_data, f, indent=2)

            # 合并用户自定义配置（TOML 格式）
            if default_toml_config and default_toml_config.strip():
                try:
                    custom_config = tomli.loads(default_toml_config)
                    _deep_merge(data, custom_config)
                except tomli.TOMLDecodeError as e:
                    logger.warning(f"Codex 自定义配置解析失败（非有效 TOML）: {e}")
        else:
            # 禁用时移除自定义 provider 配置
            if "model_providers" in data:
                data["model_providers"].pop(CODEX_PROVIDER_KEY, None)
            if data.get("model_provider") == CODEX_PROVIDER_KEY:
                data.pop("model_provider", None)
            # 移除 auth.json 中的 OPENAI_API_KEY 字段
            if auth_path.exists():
                try:
                    with open(auth_path, 'r', encoding='utf-8') as f:
                        auth_data = json.load(f)
                    auth_data.pop(CODEX_AUTH_KEY, None)
                    with open(auth_path, 'w', encoding='utf-8') as f:
                        json.dump(auth_data, f, indent=2)
                except (json.JSONDecodeError, IOError):
                    pass
            # 移除自定义配置中的字段
            if default_toml_config and default_toml_config.strip():
                try:
                    custom_config = tomli.loads(default_toml_config)
                    _remove_keys(data, custom_config)
                except tomli.TOMLDecodeError:
                    pass

        with open(config_path, 'wb') as f:
            tomli_w.dump(data, f)

        logger.info(f"已同步 Codex 配置 (enabled={enabled})")
        return True
    except Exception as e:
        logger.error(f"同步 Codex 配置失败: {e}")
        return False


def _deep_merge(base: dict, override: dict) -> None:
    """深度合并字典，override 覆盖 base"""
    for key, value in override.items():
        if key in base and isinstance(base[key], dict) and isinstance(value, dict):
            _deep_merge(base[key], value)
        else:
            base[key] = value


def _remove_keys(base: dict, keys_to_remove: dict) -> None:
    """从 base 中移除 keys_to_remove 指定的键"""
    for key, value in keys_to_remove.items():
        if key in base:
            if isinstance(value, dict) and isinstance(base[key], dict):
                _remove_keys(base[key], value)
                if not base[key]:
                    del base[key]
            else:
                del base[key]


def sync_gemini_settings(base_url: str, api_key: str, default_json_config: str, enabled: bool) -> bool:
    """同步 Gemini CLI 配置"""
    settings_path = get_gemini_settings_path()

    if not settings_path.parent.exists():
        logger.debug("Gemini CLI 未安装，跳过配置同步")
        return True

    try:
        # 读取现有配置，处理文件为空的情况
        data = {}
        if settings_path.exists():
            content = settings_path.read_text(encoding='utf-8').strip()
            if content:
                data = json.loads(content)

        env_path = settings_path.parent / ".env"
        if enabled:
            env_lines = [
                f'GEMINI_API_KEY={api_key}',
                f'GOOGLE_GEMINI_BASE_URL={base_url}',
                ''
            ]
            with open(env_path, 'w', encoding='utf-8') as f:
                f.write('\n'.join(env_lines))

            # 设置认证类型
            if "security" not in data:
                data["security"] = {}
            if "auth" not in data["security"]:
                data["security"]["auth"] = {}
            data["security"]["auth"]["selectedType"] = "gemini-api-key"

            # 合并用户自定义配置（仅当配置非空时）
            if default_json_config and default_json_config.strip():
                try:
                    custom_config = json.loads(default_json_config)
                    _deep_merge(data, custom_config)
                except json.JSONDecodeError as e:
                    logger.warning(f"Gemini CLI 自定义配置解析失败（非有效 JSON）: {e}")

            with open(settings_path, 'w', encoding='utf-8') as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
        else:
            # 禁用时清空 .env 文件
            if env_path.exists():
                with open(env_path, 'w', encoding='utf-8') as f:
                    f.write("")
            # 移除 settings.json 中的认证配置
            if settings_path.exists():
                if "security" in data and "auth" in data["security"]:
                    data["security"]["auth"].pop("selectedType", None)
                    # 如果 auth 为空，移除 auth
                    if not data["security"]["auth"]:
                        data["security"].pop("auth", None)
                    # 如果 security 为空，移除 security
                    if not data["security"]:
                        data.pop("security", None)
                # 移除自定义配置中的字段
                if default_json_config and default_json_config.strip():
                    try:
                        custom_config = json.loads(default_json_config)
                        _remove_keys(data, custom_config)
                    except json.JSONDecodeError:
                        pass
                with open(settings_path, 'w', encoding='utf-8') as f:
                    json.dump(data, f, indent=2, ensure_ascii=False)

        logger.info(f"已同步 Gemini CLI 配置 (enabled={enabled})")
        return True
    except Exception as e:
        logger.error(f"同步 Gemini CLI 配置失败: {e}")
        return False


def sync_cli_settings(cli_type: str, base_url: str, api_key: str, default_json_config: str, enabled: bool) -> bool:
    """同步指定 CLI 的配置"""
    if cli_type == "claude_code":
        return sync_claude_settings(base_url, api_key, default_json_config, enabled)
    elif cli_type == "codex":
        return sync_codex_settings(base_url, api_key, default_json_config, enabled)
    elif cli_type == "gemini":
        return sync_gemini_settings(base_url, api_key, default_json_config, enabled)
    else:
        logger.warning(f"未知的 CLI 类型: {cli_type}")
        return False
