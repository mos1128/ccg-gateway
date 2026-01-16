import os
import sys
import threading
from PIL import Image
import pystray


class TrayIcon:
    def __init__(self, on_show, on_quit):
        self.on_show = on_show
        self.on_quit = on_quit
        self.icon = None
        self._minimize_on_close = True

    @property
    def minimize_on_close(self):
        return self._minimize_on_close

    @minimize_on_close.setter
    def minimize_on_close(self, value):
        self._minimize_on_close = value
        if self.icon:
            self.icon.update_menu()

    def _load_icon_image(self):
        icon_path = os.path.join(sys._MEIPASS, "desktop", "icon.ico")
        return Image.open(icon_path)

    def _toggle_close_behavior(self):
        self._minimize_on_close = not self._minimize_on_close

    def _build_menu(self):
        return pystray.Menu(
            pystray.MenuItem("显示窗口", lambda: self.on_show()),
            pystray.MenuItem(
                "关闭时最小化到托盘",
                lambda: self._toggle_close_behavior(),
                checked=lambda _: self._minimize_on_close,
            ),
            pystray.Menu.SEPARATOR,
            pystray.MenuItem("退出", lambda: self.on_quit()),
        )

    def run(self):
        self.icon = pystray.Icon(
            "ccg-gateway",
            self._load_icon_image(),
            "CCG Gateway",
            menu=self._build_menu(),
        )
        self.icon.run()

    def stop(self):
        if self.icon:
            self.icon.stop()

    def start_in_thread(self):
        thread = threading.Thread(target=self.run, daemon=True)
        thread.start()
        return thread
