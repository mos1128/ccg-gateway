# -*- mode: python ; coding: utf-8 -*-
import os
import sys

block_cipher = None

# Paths
project_root = os.path.dirname(SPECPATH)
backend_path = os.path.join(project_root, 'backend')
frontend_dist = os.path.join(project_root, 'frontend', 'dist')

a = Analysis(
    [os.path.join(project_root, 'desktop', 'main.py')],
    pathex=[backend_path],
    binaries=[],
    datas=[
        (frontend_dist, 'frontend/dist'),
    ],
    hiddenimports=[
        'uvicorn',
        'uvicorn.logging',
        'uvicorn.loops',
        'uvicorn.loops.auto',
        'uvicorn.protocols',
        'uvicorn.protocols.http',
        'uvicorn.protocols.http.auto',
        'uvicorn.protocols.websockets',
        'uvicorn.protocols.websockets.auto',
        'uvicorn.lifespan',
        'uvicorn.lifespan.on',
        'uvicorn.lifespan.off',
        'fastapi',
        'pydantic',
        'pydantic_settings',
        'sqlalchemy',
        'sqlalchemy.ext.asyncio',
        'aiosqlite',
        'httpx',
        'webview',
        'pystray',
        'PIL',
        'tomli_w',
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name='ccg-gateway',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name='ccg-gateway',
)
