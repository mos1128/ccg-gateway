from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select
from typing import Optional
import time
import json

from app.models.models import GatewaySettings, TimeoutSettings, CliSettings, Provider
from app.schemas.schemas import (
    AllSettingsResponse, GatewaySettingsResponse, TimeoutSettingsResponse,
    CliSettingsResponse, GatewaySettingsUpdate, TimeoutSettingsUpdate, CliSettingsUpdate
)
from app.services.cli_sync_service import sync_cli_settings
from app.core.config import settings as app_settings


class SettingsService:
    def __init__(self, db: AsyncSession):
        self.db = db

    async def get_all_settings(self) -> AllSettingsResponse:
        """Get all settings."""
        gateway = await self._get_gateway_settings()
        timeouts = await self._get_timeout_settings()
        cli_settings = await self._get_all_cli_settings()

        return AllSettingsResponse(
            gateway=gateway,
            timeouts=timeouts,
            cli_settings=cli_settings
        )

    async def _get_gateway_settings(self) -> GatewaySettingsResponse:
        result = await self.db.execute(select(GatewaySettings).where(GatewaySettings.id == 1))
        settings = result.scalar_one_or_none()
        if not settings:
            return GatewaySettingsResponse(debug_log=False)
        return GatewaySettingsResponse(debug_log=bool(settings.debug_log))

    async def _get_timeout_settings(self) -> TimeoutSettingsResponse:
        result = await self.db.execute(select(TimeoutSettings).where(TimeoutSettings.id == 1))
        settings = result.scalar_one_or_none()
        if not settings:
            return TimeoutSettingsResponse(
                stream_first_byte_timeout=30,
                stream_idle_timeout=60,
                non_stream_timeout=120
            )
        return TimeoutSettingsResponse(
            stream_first_byte_timeout=settings.stream_first_byte_timeout,
            stream_idle_timeout=settings.stream_idle_timeout,
            non_stream_timeout=settings.non_stream_timeout
        )

    async def _get_all_cli_settings(self) -> dict[str, CliSettingsResponse]:
        result = await self.db.execute(select(CliSettings))
        settings = result.scalars().all()
        return {
            s.cli_type: CliSettingsResponse(
                cli_type=s.cli_type,
                enabled=bool(s.enabled),
                default_json_config=s.default_json_config
            ) for s in settings
        }

    async def get_cli_settings(self, cli_type: str) -> Optional[CliSettingsResponse]:
        result = await self.db.execute(
            select(CliSettings).where(CliSettings.cli_type == cli_type)
        )
        settings = result.scalar_one_or_none()
        if not settings:
            return None
        return CliSettingsResponse(
            cli_type=settings.cli_type,
            enabled=bool(settings.enabled),
            default_json_config=settings.default_json_config
        )

    async def update_gateway_settings(self, data: GatewaySettingsUpdate):
        now = int(time.time())
        result = await self.db.execute(select(GatewaySettings).where(GatewaySettings.id == 1))
        settings = result.scalar_one_or_none()

        if not settings:
            settings = GatewaySettings(id=1, updated_at=now)
            self.db.add(settings)

        if data.debug_log is not None:
            settings.debug_log = 1 if data.debug_log else 0
        settings.updated_at = now

        await self.db.commit()

    async def update_timeout_settings(self, data: TimeoutSettingsUpdate):
        now = int(time.time())
        result = await self.db.execute(select(TimeoutSettings).where(TimeoutSettings.id == 1))
        settings = result.scalar_one_or_none()

        if not settings:
            settings = TimeoutSettings(id=1, updated_at=now)
            self.db.add(settings)

        if data.stream_first_byte_timeout is not None:
            settings.stream_first_byte_timeout = data.stream_first_byte_timeout
        if data.stream_idle_timeout is not None:
            settings.stream_idle_timeout = data.stream_idle_timeout
        if data.non_stream_timeout is not None:
            settings.non_stream_timeout = data.non_stream_timeout
        settings.updated_at = now

        await self.db.commit()

    async def update_cli_settings(self, cli_type: str, data: CliSettingsUpdate):
        # 验证配置格式
        if data.default_json_config is not None and data.default_json_config.strip():
            config = data.default_json_config.strip()

            # 对于 claude_code 和 gemini，验证 JSON 格式
            if cli_type in ('claude_code', 'gemini'):
                try:
                    json.loads(config)
                except json.JSONDecodeError as e:
                    raise ValueError(f"JSON 格式错误: {str(e)}")

            # 对于 codex，验证 TOML 格式
            elif cli_type == 'codex':
                try:
                    import tomli
                    tomli.loads(config)
                except ImportError:
                    pass  # tomli 未安装，跳过验证
                except Exception as e:
                    raise ValueError(f"TOML 格式错误: {str(e)}")

        now = int(time.time())
        result = await self.db.execute(
            select(CliSettings).where(CliSettings.cli_type == cli_type)
        )
        settings = result.scalar_one_or_none()

        old_enabled = bool(settings.enabled) if settings else False

        if not settings:
            settings = CliSettings(cli_type=cli_type, updated_at=now)
            self.db.add(settings)

        if data.enabled is not None:
            settings.enabled = 1 if data.enabled else 0
        if data.default_json_config is not None:
            settings.default_json_config = data.default_json_config
        settings.updated_at = now

        await self.db.commit()

        # 如果 enabled 状态发生变化，同步配置到 CLI
        new_enabled = bool(settings.enabled)
        if new_enabled != old_enabled or (new_enabled and data.default_json_config is not None):
            await self._sync_cli_config(cli_type, settings, new_enabled)

    async def _sync_cli_config(self, cli_type: str, settings: CliSettings, enabled: bool):
        """同步配置到 CLI"""
        base_url = f"http://127.0.0.1:{app_settings.GATEWAY_PORT}"
        api_key = "ccg-gateway"

        sync_cli_settings(cli_type, base_url, api_key, settings.default_json_config, enabled)
