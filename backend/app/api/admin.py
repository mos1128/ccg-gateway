from fastapi import APIRouter

from app.api.v1 import providers, settings as settings_api, mcp, prompts, stats, logs, sessions

admin_router = APIRouter()

admin_router.include_router(providers.router, prefix="/providers", tags=["providers"])
admin_router.include_router(settings_api.router, prefix="/settings", tags=["settings"])
admin_router.include_router(mcp.router, prefix="/mcp", tags=["mcp"])
admin_router.include_router(prompts.router, prefix="/prompts", tags=["prompts"])
admin_router.include_router(stats.router, prefix="/stats", tags=["stats"])
admin_router.include_router(logs.router, prefix="/logs", tags=["logs"])
admin_router.include_router(sessions.router, prefix="/sessions", tags=["sessions"])
