from fastapi import APIRouter, HTTPException, Query

from app.services.session_service import SessionService, PaginatedProjects, PaginatedSessions

router = APIRouter()
session_service = SessionService()


@router.get("/projects", response_model=PaginatedProjects)
async def list_projects(
    cli_type: str = Query(..., description="CLI type: claude_code, codex, gemini"),
    page: int = Query(1, ge=1, description="Page number"),
    page_size: int = Query(20, ge=1, le=100, description="Items per page")
):
    """List projects with pagination."""
    if cli_type not in ("claude_code", "codex", "gemini"):
        raise HTTPException(status_code=400, detail="Invalid cli_type")
    return session_service.list_projects(cli_type, page, page_size)


@router.get("/projects/{project_name}/sessions", response_model=PaginatedSessions)
async def list_sessions(
    project_name: str,
    cli_type: str = Query(..., description="CLI type: claude_code, codex, gemini"),
    page: int = Query(1, ge=1, description="Page number"),
    page_size: int = Query(20, ge=1, le=100, description="Items per page")
):
    """List sessions with pagination."""
    if cli_type not in ("claude_code", "codex", "gemini"):
        raise HTTPException(status_code=400, detail="Invalid cli_type")
    return session_service.list_sessions(cli_type, project_name, page, page_size)


@router.get("/projects/{project_name}/sessions/{session_id}/messages")
async def get_session_messages(
    project_name: str,
    session_id: str,
    cli_type: str = Query(..., description="CLI type: claude_code, codex, gemini")
):
    """Get all messages from a session."""
    if cli_type not in ("claude_code", "codex", "gemini"):
        raise HTTPException(status_code=400, detail="Invalid cli_type")
    return session_service.get_session_messages(cli_type, project_name, session_id)


@router.delete("/projects/{project_name}/sessions/{session_id}")
async def delete_session(
    project_name: str,
    session_id: str,
    cli_type: str = Query(..., description="CLI type: claude_code, codex, gemini")
):
    """Delete a session."""
    if cli_type not in ("claude_code", "codex", "gemini"):
        raise HTTPException(status_code=400, detail="Invalid cli_type")
    success = session_service.delete_session(cli_type, project_name, session_id)
    if not success:
        raise HTTPException(status_code=404, detail="Session not found")
    return {"success": True}


@router.delete("/projects/{project_name}")
async def delete_project(
    project_name: str,
    cli_type: str = Query(..., description="CLI type: claude_code, codex, gemini")
):
    """Delete a project and all its sessions."""
    if cli_type not in ("claude_code", "codex", "gemini"):
        raise HTTPException(status_code=400, detail="Invalid cli_type")
    success = session_service.delete_project(cli_type, project_name)
    if not success:
        raise HTTPException(status_code=404, detail="Project not found")
    return {"success": True}
