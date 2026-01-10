from sqlalchemy import Column, Integer, String, Text, ForeignKey, UniqueConstraint, CheckConstraint
from sqlalchemy.orm import relationship
from app.core.database import Base


class Provider(Base):
    __tablename__ = "providers"

    id = Column(Integer, primary_key=True, autoincrement=True)
    cli_type = Column(String(20), nullable=False, default="claude_code")
    name = Column(String(100), nullable=False)
    base_url = Column(String(500), nullable=False)
    api_key = Column(String(500), nullable=False)
    enabled = Column(Integer, nullable=False, default=1)
    failure_threshold = Column(Integer, nullable=False, default=3)
    blacklist_minutes = Column(Integer, nullable=False, default=10)
    consecutive_failures = Column(Integer, nullable=False, default=0)
    blacklisted_until = Column(Integer, nullable=True)
    sort_order = Column(Integer, nullable=False, default=0)
    created_at = Column(Integer, nullable=False)
    updated_at = Column(Integer, nullable=False)

    model_maps = relationship("ProviderModelMap", back_populates="provider", cascade="all, delete-orphan", order_by="ProviderModelMap.id")

    __table_args__ = (
        UniqueConstraint("cli_type", "name", name="uq_cli_provider_name"),
        CheckConstraint("cli_type IN ('claude_code','codex','gemini')", name="ck_provider_cli_type"),
    )


class ProviderModelMap(Base):
    __tablename__ = "provider_model_map"

    id = Column(Integer, primary_key=True, autoincrement=True)
    provider_id = Column(Integer, ForeignKey("providers.id", ondelete="CASCADE"), nullable=False)
    source_model = Column(String(100), nullable=False)  # CLI请求的模型名
    target_model = Column(String(100), nullable=False)  # 转发给服务商的模型名
    enabled = Column(Integer, nullable=False, default=1)

    provider = relationship("Provider", back_populates="model_maps")

    __table_args__ = (
        UniqueConstraint("provider_id", "source_model", name="uq_provider_source_model"),
    )


class GatewaySettings(Base):
    __tablename__ = "gateway_settings"

    id = Column(Integer, primary_key=True, default=1)
    debug_log = Column(Integer, nullable=False, default=0)
    updated_at = Column(Integer, nullable=False)

    __table_args__ = (
        CheckConstraint("id = 1", name="ck_gateway_singleton"),
    )


class TimeoutSettings(Base):
    __tablename__ = "timeout_settings"

    id = Column(Integer, primary_key=True, default=1)
    stream_first_byte_timeout = Column(Integer, nullable=False, default=30)
    stream_idle_timeout = Column(Integer, nullable=False, default=60)
    non_stream_timeout = Column(Integer, nullable=False, default=120)
    updated_at = Column(Integer, nullable=False)

    __table_args__ = (
        CheckConstraint("id = 1", name="ck_timeout_singleton"),
    )


class CliSettings(Base):
    __tablename__ = "cli_settings"

    cli_type = Column(String(20), primary_key=True)  # claude_code, codex, gemini
    enabled = Column(Integer, nullable=False, default=0)
    default_json_config = Column(Text, nullable=False, default="{}")
    updated_at = Column(Integer, nullable=False)

    __table_args__ = (
        CheckConstraint("cli_type IN ('claude_code','codex','gemini')", name="ck_cli_type"),
    )


class McpConfig(Base):
    __tablename__ = "mcp_configs"

    id = Column(Integer, primary_key=True, autoincrement=True)
    name = Column(String(100), unique=True, nullable=False)
    config_json = Column(Text, nullable=False)
    enabled = Column(Integer, nullable=False, default=1)
    updated_at = Column(Integer, nullable=False)

    cli_flags = relationship("McpCliFlag", back_populates="mcp", cascade="all, delete-orphan")


class McpCliFlag(Base):
    __tablename__ = "mcp_cli_flags"

    mcp_id = Column(Integer, ForeignKey("mcp_configs.id", ondelete="CASCADE"), primary_key=True)
    cli_type = Column(String(20), primary_key=True)
    enabled = Column(Integer, nullable=False, default=1)

    mcp = relationship("McpConfig", back_populates="cli_flags")


class PromptPreset(Base):
    __tablename__ = "prompt_presets"

    id = Column(Integer, primary_key=True, autoincrement=True)
    name = Column(String(100), unique=True, nullable=False)
    content = Column(Text, nullable=False)
    enabled = Column(Integer, nullable=False, default=1)
    updated_at = Column(Integer, nullable=False)

    cli_flags = relationship("PromptCliFlag", back_populates="prompt", cascade="all, delete-orphan")


class PromptCliFlag(Base):
    __tablename__ = "prompt_cli_flags"

    prompt_id = Column(Integer, ForeignKey("prompt_presets.id", ondelete="CASCADE"), primary_key=True)
    cli_type = Column(String(20), primary_key=True)
    enabled = Column(Integer, nullable=False, default=1)

    prompt = relationship("PromptPreset", back_populates="cli_flags")


class UsageDaily(Base):
    __tablename__ = "usage_daily"

    id = Column(Integer, primary_key=True, autoincrement=True)
    usage_date = Column(String(10), nullable=False)  # YYYY-MM-DD
    provider_id = Column(Integer, ForeignKey("providers.id", ondelete="CASCADE"), nullable=False)
    cli_type = Column(String(20), nullable=False)
    request_count = Column(Integer, nullable=False, default=0)
    success_count = Column(Integer, nullable=False, default=0)
    failure_count = Column(Integer, nullable=False, default=0)
    prompt_tokens = Column(Integer, nullable=False, default=0)
    completion_tokens = Column(Integer, nullable=False, default=0)

    __table_args__ = (
        UniqueConstraint("usage_date", "provider_id", "cli_type", name="uq_usage_daily"),
    )


class RequestLog(Base):
    __tablename__ = "request_logs"

    id = Column(Integer, primary_key=True, autoincrement=True)
    created_at = Column(Integer, nullable=False)
    cli_type = Column(String(20), nullable=False)
    provider_name = Column(String(100), nullable=False)
    success = Column(Integer, nullable=False, default=1)
    status_code = Column(Integer, nullable=True)
    elapsed_ms = Column(Integer, nullable=False, default=0)
    # CLI request
    client_method = Column(String(10), nullable=False)
    client_path = Column(String(500), nullable=False)
    client_headers = Column(Text, nullable=False)
    client_body = Column(Text, nullable=False)
    # Gateway forward request
    forward_url = Column(String(1000), nullable=False)
    forward_headers = Column(Text, nullable=False)
    forward_body = Column(Text, nullable=False)
    # Provider response
    provider_status = Column(Integer, nullable=True)
    provider_headers = Column(Text, nullable=True)
    provider_body = Column(Text, nullable=True)
    # Gateway forward response
    response_status = Column(Integer, nullable=True)
    response_headers = Column(Text, nullable=True)
    response_body = Column(Text, nullable=True)
    error_message = Column(Text, nullable=True)


class SystemLog(Base):
    __tablename__ = "system_logs"

    id = Column(Integer, primary_key=True, autoincrement=True)
    created_at = Column(Integer, nullable=False)
    level = Column(String(10), nullable=False)  # INFO, WARN, ERROR
    event_type = Column(String(50), nullable=False)  # provider_failure, provider_blacklist, provider_switch, etc.
    provider_name = Column(String(100), nullable=True)
    message = Column(Text, nullable=False)
    details = Column(Text, nullable=True)  # JSON details
