from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.ext.asyncio import AsyncSession

from app.core.database import get_db
from app.schemas.schemas import (
    AllSettingsResponse, GatewaySettingsUpdate, TimeoutSettingsUpdate,
    CliSettingsUpdate, CliType, SystemStatusResponse
)
from app.services.settings_service import SettingsService
from app.core.uptime import get_uptime
from app.core.config import settings as app_settings

router = APIRouter()


@router.get("", response_model=AllSettingsResponse)
async def get_all_settings(db: AsyncSession = Depends(get_db)):
    service = SettingsService(db)
    return await service.get_all_settings()


@router.put("/gateway")
async def update_gateway_settings(data: GatewaySettingsUpdate, db: AsyncSession = Depends(get_db)):
    service = SettingsService(db)
    await service.update_gateway_settings(data)
    return {"message": "Gateway settings updated"}


@router.put("/timeouts")
async def update_timeout_settings(data: TimeoutSettingsUpdate, db: AsyncSession = Depends(get_db)):
    service = SettingsService(db)
    await service.update_timeout_settings(data)
    return {"message": "Timeout settings updated"}


@router.get("/cli/{cli_type}")
async def get_cli_settings(cli_type: CliType, db: AsyncSession = Depends(get_db)):
    service = SettingsService(db)
    result = await service.get_cli_settings(cli_type.value)
    if not result:
        raise HTTPException(status_code=404, detail="CLI settings not found")
    return result


@router.put("/cli/{cli_type}")
async def update_cli_settings(cli_type: CliType, data: CliSettingsUpdate, db: AsyncSession = Depends(get_db)):
    service = SettingsService(db)
    try:
        await service.update_cli_settings(cli_type.value, data)
        return {"message": "CLI settings updated"}
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))


@router.get("/status", response_model=SystemStatusResponse)
async def get_system_status():
    return SystemStatusResponse(
        status="running",
        port=app_settings.GATEWAY_PORT,
        uptime=get_uptime(),
        version=app_settings.VERSION
    )
