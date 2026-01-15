from pydantic import BaseModel, Field
from typing import Optional, Literal
from enum import Enum


class CliType(str, Enum):
    CLAUDE_CODE = "claude_code"
    CODEX = "codex"
    GEMINI = "gemini"


# Provider Schemas
class ModelMapCreate(BaseModel):
    source_model: str = Field(..., min_length=1)
    target_model: str = Field(..., min_length=1)
    enabled: bool = True


class ModelMapResponse(BaseModel):
    id: int
    source_model: str
    target_model: str
    enabled: bool

    class Config:
        from_attributes = True


class ProviderCreate(BaseModel):
    cli_type: CliType = CliType.CLAUDE_CODE
    name: str = Field(..., min_length=1, max_length=100)
    base_url: str = Field(..., min_length=1)
    api_key: str = Field(..., min_length=1)
    enabled: bool = True
    failure_threshold: int = Field(default=3, ge=1)
    blacklist_minutes: int = Field(default=10, ge=0)
    model_maps: list[ModelMapCreate] = []


class ProviderUpdate(BaseModel):
    name: Optional[str] = Field(None, min_length=1, max_length=100)
    base_url: Optional[str] = None
    api_key: Optional[str] = None
    enabled: Optional[bool] = None
    failure_threshold: Optional[int] = Field(None, ge=1)
    blacklist_minutes: Optional[int] = Field(None, ge=0)
    model_maps: Optional[list[ModelMapCreate]] = None


class ProviderResponse(BaseModel):
    id: int
    cli_type: CliType
    name: str
    base_url: str
    api_key: str = ""
    enabled: bool
    failure_threshold: int
    blacklist_minutes: int
    consecutive_failures: int
    blacklisted_until: Optional[int]
    sort_order: int
    model_maps: list[ModelMapResponse] = []
    is_blacklisted: bool = False

    class Config:
        from_attributes = True


class ProviderReorder(BaseModel):
    ids: list[int]


# Settings Schemas
class GatewaySettingsResponse(BaseModel):
    debug_log: bool

    class Config:
        from_attributes = True


class GatewaySettingsUpdate(BaseModel):
    debug_log: Optional[bool] = None


class TimeoutSettingsResponse(BaseModel):
    stream_first_byte_timeout: int
    stream_idle_timeout: int
    non_stream_timeout: int

    class Config:
        from_attributes = True


class TimeoutSettingsUpdate(BaseModel):
    stream_first_byte_timeout: Optional[int] = Field(None, ge=1)
    stream_idle_timeout: Optional[int] = Field(None, ge=1)
    non_stream_timeout: Optional[int] = Field(None, ge=1)


class CliSettingsResponse(BaseModel):
    cli_type: CliType
    enabled: bool
    default_json_config: str

    class Config:
        from_attributes = True


class CliSettingsUpdate(BaseModel):
    enabled: Optional[bool] = None
    default_json_config: Optional[str] = None


class AllSettingsResponse(BaseModel):
    gateway: GatewaySettingsResponse
    timeouts: TimeoutSettingsResponse
    cli_settings: dict[str, CliSettingsResponse]


# MCP Schemas
class McpCliFlagsCreate(BaseModel):
    claude_code: bool = False
    codex: bool = False
    gemini: bool = False


class McpCreate(BaseModel):
    name: str = Field(..., min_length=1, max_length=100)
    config_json: str
    enabled: bool = True
    cli_flags: McpCliFlagsCreate = McpCliFlagsCreate()


class McpUpdate(BaseModel):
    name: Optional[str] = Field(None, min_length=1, max_length=100)
    config_json: Optional[str] = None
    enabled: Optional[bool] = None
    cli_flags: Optional[McpCliFlagsCreate] = None


class McpResponse(BaseModel):
    id: int
    name: str
    config_json: str
    enabled: bool
    cli_flags: dict[str, bool]

    class Config:
        from_attributes = True


# Prompt Schemas
class PromptCliFlagsCreate(BaseModel):
    claude_code: bool = False
    codex: bool = False
    gemini: bool = False


class PromptCreate(BaseModel):
    name: str = Field(..., min_length=1, max_length=100)
    content: str
    enabled: bool = True
    cli_flags: PromptCliFlagsCreate = PromptCliFlagsCreate()


class PromptUpdate(BaseModel):
    name: Optional[str] = Field(None, min_length=1, max_length=100)
    content: Optional[str] = None
    enabled: Optional[bool] = None
    cli_flags: Optional[PromptCliFlagsCreate] = None


class PromptResponse(BaseModel):
    id: int
    name: str
    content: str
    enabled: bool
    cli_flags: dict[str, bool]

    class Config:
        from_attributes = True


# Stats Schemas
class DailyStatsResponse(BaseModel):
    usage_date: str
    provider_id: int
    provider_name: str
    cli_type: str
    request_count: int
    success_count: int
    failure_count: int
    prompt_tokens: int
    completion_tokens: int


class ProviderStatsResponse(BaseModel):
    provider_id: int
    provider_name: str
    cli_type: str
    total_requests: int
    total_success: int
    total_failure: int
    success_rate: float
    total_tokens: int


# System Schemas
class SystemStatusResponse(BaseModel):
    status: Literal["running", "stopped"]
    port: int
    uptime: int
    version: str


# Log Schemas
class RequestLogListItem(BaseModel):
    id: int
    created_at: int
    cli_type: str
    provider_name: str
    model_id: Optional[str]
    success: bool
    status_code: Optional[int]
    elapsed_ms: int
    client_method: str
    client_path: str

    class Config:
        from_attributes = True


class RequestLogDetail(BaseModel):
    id: int
    created_at: int
    cli_type: str
    provider_name: str
    model_id: Optional[str]
    success: bool
    status_code: Optional[int]
    elapsed_ms: int
    client_method: str
    client_path: str
    client_headers: str
    client_body: str
    forward_url: str
    forward_headers: str
    forward_body: str
    provider_status: Optional[int]
    provider_headers: Optional[str]
    provider_body: Optional[str]
    response_status: Optional[int]
    response_headers: Optional[str]
    response_body: Optional[str]
    error_message: Optional[str]

    class Config:
        from_attributes = True


class RequestLogListResponse(BaseModel):
    items: list[RequestLogListItem]
    total: int
    page: int
    page_size: int


class SystemLogItem(BaseModel):
    id: int
    created_at: int
    level: str
    event_type: str
    provider_name: Optional[str]
    message: str
    details: Optional[str]

    class Config:
        from_attributes = True


class SystemLogListResponse(BaseModel):
    items: list[SystemLogItem]
    total: int
    page: int
    page_size: int


class ClearLogsRequest(BaseModel):
    before_timestamp: Optional[int] = None
