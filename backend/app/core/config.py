import sys
from pathlib import Path
from pydantic_settings import BaseSettings


IS_PACKAGED = getattr(sys, 'frozen', False)


def get_base_dir() -> Path:
    if IS_PACKAGED:
        return Path(sys.executable).parent
    return Path(__file__).resolve().parent.parent.parent.parent


def get_data_dir() -> Path:
    return get_base_dir() / "data"


def get_env_file() -> Path:
    return get_base_dir() / ".env"


def get_frontend_dist() -> Path | None:
    """
    获取前端静态文件目录路径

    桌面模式（打包后）：返回临时目录中的前端文件路径
    开发模式：返回 None（前端应使用 Vite 开发服务器，支持热更新）
    """
    if IS_PACKAGED:
        # 桌面模式：提供静态文件服务
        meipass = getattr(sys, '_MEIPASS', None)
        if meipass:
            return Path(meipass) / "frontend" / "dist"
    # 开发模式：不提供静态文件服务，前端使用 Vite 开发服务器
    return None


class Settings(BaseSettings):
    PROJECT_NAME: str = "CCG-Gateway"
    VERSION: str = "0.1.0"

    # Gateway defaults
    GATEWAY_PORT: int = 7788
    GATEWAY_HOST: str = "127.0.0.1"

    # Timeout defaults (seconds)
    STREAM_FIRST_BYTE_TIMEOUT: int = 30
    STREAM_IDLE_TIMEOUT: int = 60
    NON_STREAM_TIMEOUT: int = 120

    # Logging
    LOG_TO_FILE: bool = False

    class Config:
        env_file = get_env_file()
        case_sensitive = True
        extra = "ignore"


settings = Settings()

# Ensure data directory exists
DATA_DIR = get_data_dir()
DATA_DIR.mkdir(exist_ok=True)
