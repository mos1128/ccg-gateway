from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select, delete, desc
from typing import List, Optional
import time
import json

from app.models.models import RequestLog, SystemLog, GatewaySettings


class LogService:
    def __init__(self, db: AsyncSession):
        self.db = db

    async def is_logging_enabled(self) -> bool:
        result = await self.db.execute(select(GatewaySettings).where(GatewaySettings.id == 1))
        settings = result.scalar_one_or_none()
        return bool(settings.debug_log) if settings else False

    async def create_request_log(
        self,
        cli_type: str,
        provider_name: str,
        success: bool,
        status_code: Optional[int],
        elapsed_ms: int,
        client_method: str,
        client_path: str,
        client_headers: dict,
        client_body: str,
        forward_url: str,
        forward_headers: dict,
        forward_body: str,
        created_at: Optional[int] = None,
        model_id: Optional[str] = None,
        provider_status: Optional[int] = None,
        provider_headers: Optional[dict] = None,
        provider_body: Optional[str] = None,
        response_status: Optional[int] = None,
        response_headers: Optional[dict] = None,
        response_body: Optional[str] = None,
        error_message: Optional[str] = None
    ) -> RequestLog:
        log = RequestLog(
            created_at=created_at or int(time.time()),
            cli_type=cli_type,
            provider_name=provider_name,
            model_id=model_id,
            success=1 if success else 0,
            status_code=status_code,
            elapsed_ms=elapsed_ms,
            client_method=client_method,
            client_path=client_path,
            client_headers=json.dumps(client_headers, ensure_ascii=False),
            client_body=client_body,
            forward_url=forward_url,
            forward_headers=json.dumps(forward_headers, ensure_ascii=False),
            forward_body=forward_body,
            provider_status=provider_status,
            provider_headers=json.dumps(provider_headers, ensure_ascii=False) if provider_headers else None,
            provider_body=provider_body,
            response_status=response_status,
            response_headers=json.dumps(response_headers, ensure_ascii=False) if response_headers else None,
            response_body=response_body,
            error_message=error_message
        )
        self.db.add(log)
        await self.db.commit()
        return log

    async def create_system_log(
        self,
        level: str,
        event_type: str,
        message: str,
        provider_name: Optional[str] = None,
        details: Optional[dict] = None
    ) -> SystemLog:
        log = SystemLog(
            created_at=int(time.time()),
            level=level,
            event_type=event_type,
            provider_name=provider_name,
            message=message,
            details=json.dumps(details, ensure_ascii=False) if details else None
        )
        self.db.add(log)
        await self.db.commit()
        return log

    async def list_request_logs(
        self,
        page: int = 1,
        page_size: int = 20,
        cli_type: Optional[str] = None,
        provider_name: Optional[str] = None,
        success: Optional[bool] = None
    ) -> tuple[List[RequestLog], int]:
        query = select(RequestLog)
        count_query = select(RequestLog)

        if cli_type:
            query = query.where(RequestLog.cli_type == cli_type)
            count_query = count_query.where(RequestLog.cli_type == cli_type)
        if provider_name:
            query = query.where(RequestLog.provider_name == provider_name)
            count_query = count_query.where(RequestLog.provider_name == provider_name)
        if success is not None:
            query = query.where(RequestLog.success == (1 if success else 0))
            count_query = count_query.where(RequestLog.success == (1 if success else 0))

        # Count total
        count_result = await self.db.execute(count_query)
        total = len(count_result.scalars().all())

        # Get paginated results
        query = query.order_by(desc(RequestLog.created_at), desc(RequestLog.id))
        query = query.offset((page - 1) * page_size).limit(page_size)
        result = await self.db.execute(query)
        logs = result.scalars().all()

        return list(logs), total

    async def get_request_log(self, log_id: int) -> Optional[RequestLog]:
        result = await self.db.execute(select(RequestLog).where(RequestLog.id == log_id))
        return result.scalar_one_or_none()

    async def list_system_logs(
        self,
        page: int = 1,
        page_size: int = 20,
        level: Optional[str] = None,
        event_type: Optional[str] = None,
        provider_name: Optional[str] = None
    ) -> tuple[List[SystemLog], int]:
        query = select(SystemLog)
        count_query = select(SystemLog)

        if level:
            query = query.where(SystemLog.level == level)
            count_query = count_query.where(SystemLog.level == level)
        if event_type:
            query = query.where(SystemLog.event_type == event_type)
            count_query = count_query.where(SystemLog.event_type == event_type)
        if provider_name:
            query = query.where(SystemLog.provider_name == provider_name)
            count_query = count_query.where(SystemLog.provider_name == provider_name)

        # Count total
        count_result = await self.db.execute(count_query)
        total = len(count_result.scalars().all())

        # Get paginated results
        query = query.order_by(desc(SystemLog.created_at), desc(SystemLog.id))
        query = query.offset((page - 1) * page_size).limit(page_size)
        result = await self.db.execute(query)
        logs = result.scalars().all()

        return list(logs), total

    async def clear_request_logs(self, before_timestamp: Optional[int] = None) -> int:
        if before_timestamp:
            result = await self.db.execute(
                delete(RequestLog).where(RequestLog.created_at < before_timestamp)
            )
        else:
            result = await self.db.execute(delete(RequestLog))
        await self.db.commit()
        return result.rowcount

    async def clear_system_logs(self, before_timestamp: Optional[int] = None) -> int:
        if before_timestamp:
            result = await self.db.execute(
                delete(SystemLog).where(SystemLog.created_at < before_timestamp)
            )
        else:
            result = await self.db.execute(delete(SystemLog))
        await self.db.commit()
        return result.rowcount
