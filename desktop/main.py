import os
import sys
import socket
import threading
import time

# Add backend to path (for development mode)
backend_path = os.path.join(os.path.dirname(os.path.dirname(__file__)), "backend")
if os.path.exists(backend_path):
    sys.path.insert(0, backend_path)

import requests
import uvicorn
import webview
from desktop.tray import TrayIcon
from app.main import app as fastapi_app
from app.core.config import settings
from app.api.v1.window import set_show_callback


def is_port_in_use(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        return s.connect_ex(("127.0.0.1", port)) == 0


def activate_existing_instance(port: int) -> bool:
    try:
        resp = requests.post(f"http://127.0.0.1:{port}/admin/v1/window/show", timeout=3)
        return resp.status_code == 200
    except Exception:
        return False


class Server:
    def __init__(self, host="127.0.0.1", port=None):
        self.host = host
        self.port = port or settings.GATEWAY_PORT
        self.server = None

    def run(self):
        config = uvicorn.Config(
            fastapi_app,
            host=self.host,
            port=self.port,
            log_level="info",
        )
        self.server = uvicorn.Server(config)
        self.server.run()

    def start_in_thread(self):
        thread = threading.Thread(target=self.run, daemon=True)
        thread.start()
        return thread


class App:
    def __init__(self):
        self.window = None
        self.server = Server()
        self.tray = TrayIcon(on_show=self.show_window, on_quit=self.quit)
        set_show_callback(self.show_window)

    def show_window(self):
        if self.window:
            self.window.show()

    def hide_window(self):
        if self.window:
            self.window.hide()

    def on_closing(self):
        self.hide_window()
        return False  # Prevent window destruction

    def quit(self):
        def _exit():
            import time
            time.sleep(0.1)
            os._exit(0)
        threading.Thread(target=_exit, daemon=True).start()
        try:
            self.tray.stop()
        except Exception:
            pass
        try:
            if self.window:
                self.window.destroy()
        except Exception:
            pass

    def run(self):
        # Start FastAPI server
        self.server.start_in_thread()
        time.sleep(1)  # Wait for server to start

        # Start tray icon
        self.tray.start_in_thread()

        # Create webview window
        self.window = webview.create_window(
            "CCG Gateway",
            f"http://{self.server.host}:{self.server.port}",
            width=1200,
            height=800,
            maximized=True,
            text_select=True,
        )
        self.window.events.closing += self.on_closing

        # Start webview (blocks until window is closed)
        webview.start()


def main():
    port = settings.GATEWAY_PORT
    if is_port_in_use(port):
        activate_existing_instance(port)
        sys.exit(0)

    app = App()
    app.run()


if __name__ == "__main__":
    main()
