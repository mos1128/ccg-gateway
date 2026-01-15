from fastapi import APIRouter, Depends, Query
from sqlalchemy.ext.asyncio import AsyncSession
from typing import Optional

from app.core.database import get_db
from app.schemas.schemas import (
    RequestLogListResponse, RequestLogListItem, RequestLogDetail,
    SystemLogListResponse, SystemLogItem, ClearLogsRequest,
    GatewaySettingsResponse, GatewaySettingsUpdate
)
from app.services.log_service import LogService
from app.services.settings_service import SettingsService

router = APIRouter()


@router.get("/settings", response_model=GatewaySettingsResponse)
async def get_log_settings(db: AsyncSession = Depends(get_db)):
    service = SettingsService(db)
    return await service._get_gateway_settings()


@router.put("/settings")
async def update_log_settings(data: GatewaySettingsUpdate, db: AsyncSession = Depends(get_db)):
    service = SettingsService(db)
    await service.update_gateway_settings(data)
    return {"message": "Log settings updated"}


@router.get("/requests", response_model=RequestLogListResponse)
async def list_request_logs(
    page: int = Query(1, ge=1),
    page_size: int = Query(20, ge=1, le=100),
    cli_type: Optional[str] = None,
    provider_name: Optional[str] = None,
    success: Optional[bool] = None,
    db: AsyncSession = Depends(get_db)
):
    service = LogService(db)
    logs, total = await service.list_request_logs(page, page_size, cli_type, provider_name, success)
    items = [
        RequestLogListItem(
            id=log.id,
            created_at=log.created_at,
            cli_type=log.cli_type,
            provider_name=log.provider_name,
            model_id=log.model_id,
            success=bool(log.success),
            status_code=log.status_code,
            elapsed_ms=log.elapsed_ms,
            client_method=log.client_method,
            client_path=log.client_path
        ) for log in logs
    ]
    return RequestLogListResponse(items=items, total=total, page=page, page_size=page_size)


@router.get("/requests/{log_id}", response_model=RequestLogDetail)
async def get_request_log(log_id: int, db: AsyncSession = Depends(get_db)):
    service = LogService(db)
    log = await service.get_request_log(log_id)
    if not log:
        from fastapi import HTTPException
        raise HTTPException(status_code=404, detail="Log not found")
    return RequestLogDetail(
        id=log.id,
        created_at=log.created_at,
        cli_type=log.cli_type,
        provider_name=log.provider_name,
        model_id=log.model_id,
        success=bool(log.success),
        status_code=log.status_code,
        elapsed_ms=log.elapsed_ms,
        client_method=log.client_method,
        client_path=log.client_path,
        client_headers=log.client_headers,
        client_body=log.client_body,
        forward_url=log.forward_url,
        forward_headers=log.forward_headers,
        forward_body=log.forward_body,
        provider_status=log.provider_status,
        provider_headers=log.provider_headers,
        provider_body=log.provider_body,
        response_status=log.response_status,
        response_headers=log.response_headers,
        response_body=log.response_body,
        error_message=log.error_message
    )


@router.delete("/requests")
async def clear_request_logs(data: ClearLogsRequest, db: AsyncSession = Depends(get_db)):
    service = LogService(db)
    count = await service.clear_request_logs(data.before_timestamp)
    return {"message": f"Cleared {count} request logs"}


@router.get("/system", response_model=SystemLogListResponse)
async def list_system_logs(
    page: int = Query(1, ge=1),
    page_size: int = Query(20, ge=1, le=100),
    level: Optional[str] = None,
    event_type: Optional[str] = None,
    provider_name: Optional[str] = None,
    db: AsyncSession = Depends(get_db)
):
    service = LogService(db)
    logs, total = await service.list_system_logs(page, page_size, level, event_type, provider_name)
    items = [
        SystemLogItem(
            id=log.id,
            created_at=log.created_at,
            level=log.level,
            event_type=log.event_type,
            provider_name=log.provider_name,
            message=log.message,
            details=log.details
        ) for log in logs
    ]
    return SystemLogListResponse(items=items, total=total, page=page, page_size=page_size)


@router.delete("/system")
async def clear_system_logs(data: ClearLogsRequest, db: AsyncSession = Depends(get_db)):
    service = LogService(db)
    count = await service.clear_system_logs(data.before_timestamp)
    return {"message": f"Cleared {count} system logs"}
