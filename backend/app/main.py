import asyncio
import logging
from contextlib import asynccontextmanager
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse

from app.core.config import settings, get_data_dir, get_frontend_dist


def setup_logging():
    handlers = [logging.StreamHandler()]
    if settings.LOG_TO_FILE:
        log_file = get_data_dir() / "app.log"
        handlers.append(logging.FileHandler(log_file, encoding='utf-8'))
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
        handlers=handlers,
    )


setup_logging()
from app.core.database import init_db, close_db
from app.core.uptime import init_start_time
from app.api.admin import admin_router
from app.api.proxy import proxy_router
from app.services.init_service import init_default_data


frontend_dist = get_frontend_dist()


@asynccontextmanager
async def lifespan(app: FastAPI):
    init_start_time()
    await init_db()
    await init_default_data()
    try:
        yield
    except asyncio.CancelledError:
        pass
    finally:
        await close_db()


app = FastAPI(
    title=settings.PROJECT_NAME,
    version=settings.VERSION,
    lifespan=lifespan
)

# CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Routes
app.include_router(admin_router, prefix="/admin/v1")


@app.get("/health")
async def health_check():
    return {"status": "ok"}


# Desktop mode: serve frontend static files
# Development mode: frontend_dist is None, use Vite dev server (pnpm dev) for hot reload
if frontend_dist:
    app.mount("/assets", StaticFiles(directory=frontend_dist / "assets"), name="assets")

    @app.get("/")
    async def serve_index():
        return FileResponse(frontend_dist / "index.html")

    @app.get("/{full_path:path}")
    async def serve_spa(full_path: str):
        file_path = frontend_dist / full_path
        if file_path.is_file():
            return FileResponse(file_path)
        return FileResponse(frontend_dist / "index.html")

# Always include proxy router for CLI forwarding
app.include_router(proxy_router)
