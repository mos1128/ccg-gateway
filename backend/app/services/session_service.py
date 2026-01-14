import os
import json
import platform
import shutil
from pathlib import Path
from typing import Optional
from pydantic import BaseModel


class SessionInfo(BaseModel):
    session_id: str
    size: int
    mtime: float
    first_message: str = ""
    git_branch: str = ""
    summary: str = ""


class ProjectInfo(BaseModel):
    name: str
    display_name: str
    full_path: str
    session_count: int
    total_size: int
    last_modified: float


class PaginatedProjects(BaseModel):
    items: list[ProjectInfo]
    total: int
    page: int
    page_size: int


class PaginatedSessions(BaseModel):
    items: list[SessionInfo]
    total: int
    page: int
    page_size: int


class SessionService:
    CLI_DIRS = {
        "claude_code": ".claude",
        "codex": ".codex",
        "gemini": ".gemini"
    }

    def __init__(self):
        self.home_dir = Path.home()

    def _get_projects_dir(self, cli_type: str) -> Path:
        cli_dir = self.CLI_DIRS.get(cli_type, ".claude")
        return self.home_dir / cli_dir / "projects"

    def _decode_project_name(self, encoded_name: str) -> tuple[str, str]:
        """Decode project directory name to display name and full path."""
        if encoded_name.startswith("-"):
            parts = encoded_name[1:].split("-")
            if platform.system() == "Windows":
                if len(parts) >= 2 and len(parts[0]) == 1:
                    drive = parts[0].upper()
                    path_parts = parts[1:]
                    full_path = f"{drive}:\\" + "\\".join(path_parts)
                    display_name = path_parts[-1] if path_parts else encoded_name
                    return display_name, full_path
            else:
                full_path = "/" + "/".join(parts)
                display_name = parts[-1] if parts else encoded_name
                return display_name, full_path
        return encoded_name, encoded_name

    def list_projects(self, cli_type: str, page: int = 1, page_size: int = 20) -> PaginatedProjects:
        """List projects with pagination."""
        projects_dir = self._get_projects_dir(cli_type)
        if not projects_dir.exists():
            return PaginatedProjects(items=[], total=0, page=page, page_size=page_size)

        # Step 1: Get all project directories with basic info (fast)
        project_dirs = []
        for item in projects_dir.iterdir():
            if not item.is_dir():
                continue
            try:
                stat = item.stat()
                project_dirs.append((item, stat.st_mtime))
            except OSError:
                continue

        # Step 2: Sort by modification time
        project_dirs.sort(key=lambda x: x[1], reverse=True)
        total = len(project_dirs)

        # Step 3: Paginate
        start = (page - 1) * page_size
        end = start + page_size
        page_dirs = project_dirs[start:end]

        # Step 4: Parse detailed info only for current page
        projects = []
        for item, _ in page_dirs:
            display_name, full_path = self._decode_project_name(item.name)
            sessions = list(item.glob("*.jsonl"))
            sessions = [s for s in sessions if not s.name.startswith("agent-")]

            if not sessions:
                total -= 1
                continue

            total_size = sum(s.stat().st_size for s in sessions)
            last_modified = max(s.stat().st_mtime for s in sessions)

            projects.append(ProjectInfo(
                name=item.name,
                display_name=display_name,
                full_path=full_path,
                session_count=len(sessions),
                total_size=total_size,
                last_modified=last_modified
            ))

        return PaginatedProjects(items=projects, total=total, page=page, page_size=page_size)

    def list_sessions(self, cli_type: str, project_name: str, page: int = 1, page_size: int = 20) -> PaginatedSessions:
        """List sessions with pagination."""
        projects_dir = self._get_projects_dir(cli_type)
        project_dir = projects_dir / project_name

        if not project_dir.exists():
            return PaginatedSessions(items=[], total=0, page=page, page_size=page_size)

        # Step 1: Get all session files with basic info (fast)
        session_files = []
        for session_file in project_dir.glob("*.jsonl"):
            if session_file.name.startswith("agent-"):
                continue
            try:
                stat = session_file.stat()
                session_files.append((session_file, stat))
            except OSError:
                continue

        # Step 2: Sort by modification time
        session_files.sort(key=lambda x: x[1].st_mtime, reverse=True)
        total = len(session_files)

        # Step 3: Paginate
        start = (page - 1) * page_size
        end = start + page_size
        page_files = session_files[start:end]

        # Step 4: Parse detailed info only for current page
        sessions = []
        for session_file, stat in page_files:
            session_id = session_file.stem
            info = self._parse_session_info(session_file)
            sessions.append(SessionInfo(
                session_id=session_id,
                size=stat.st_size,
                mtime=stat.st_mtime,
                first_message=info.get("first_message", ""),
                git_branch=info.get("git_branch", ""),
                summary=info.get("summary", "")
            ))

        return PaginatedSessions(items=sessions, total=total, page=page, page_size=page_size)

    def _parse_session_info(self, file_path: Path) -> dict:
        """Parse session file to extract info."""
        result = {"first_message": "", "git_branch": "", "summary": ""}

        try:
            file_size = file_path.stat().st_size
            if file_size > 10 * 1024 * 1024:
                with open(file_path, "r", encoding="utf-8") as f:
                    head_content = f.read(32 * 1024)
                lines = head_content.split("\n")[:50]
            else:
                with open(file_path, "r", encoding="utf-8") as f:
                    lines = f.readlines()[:50]

            for line in lines:
                line = line.strip()
                if not line:
                    continue
                try:
                    data = json.loads(line)
                    if data.get("type") == "summary" and data.get("summary"):
                        result["summary"] = data["summary"]
                    if data.get("gitBranch") and not result["git_branch"]:
                        result["git_branch"] = data["gitBranch"]
                    if data.get("type") == "user" and data.get("message"):
                        msg = data["message"]
                        content = msg.get("content", "")
                        if content and content != "Warmup" and not result["first_message"]:
                            if isinstance(content, str):
                                result["first_message"] = content[:200]
                            elif isinstance(content, list):
                                for item in content:
                                    if isinstance(item, dict) and item.get("type") == "text":
                                        result["first_message"] = item.get("text", "")[:200]
                                        break
                except json.JSONDecodeError:
                    continue
        except Exception:
            pass

        return result

    def get_session_messages(self, cli_type: str, project_name: str, session_id: str) -> list[dict]:
        """Get all messages from a session."""
        projects_dir = self._get_projects_dir(cli_type)
        session_file = projects_dir / project_name / f"{session_id}.jsonl"

        if not session_file.exists():
            return []

        messages = []
        try:
            with open(session_file, "r", encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        data = json.loads(line)
                        msg_type = data.get("type")
                        if msg_type in ("user", "assistant"):
                            message = data.get("message", {})
                            content = message.get("content", "")

                            if isinstance(content, list):
                                text_parts = []
                                for item in content:
                                    if isinstance(item, dict):
                                        item_type = item.get("type")

                                        if item_type == "text":
                                            text_parts.append(item.get("text", ""))

                                        elif item_type == "tool_result" and msg_type == "user":
                                            result_content = item.get("content", "")
                                            if isinstance(result_content, str):
                                                text_parts.append(f"**[工具结果]**\n```\n{result_content}\n```")
                                            else:
                                                text_parts.append(f"**[工具结果]**\n```json\n{json.dumps(result_content, ensure_ascii=False, indent=2)}\n```")

                                        elif item_type == "tool_use" and msg_type == "assistant":
                                            tool_name = item.get("name", "unknown")
                                            tool_input = item.get("input", {})
                                            input_str = json.dumps(tool_input, ensure_ascii=False, indent=2)
                                            text_parts.append(f"**[调用工具: {tool_name}]**\n```json\n{input_str}\n```")

                                        elif item_type == "thinking" and msg_type == "assistant":
                                            thinking = item.get("thinking", "")
                                            if thinking:
                                                text_parts.append(f"**[思考]**\n{thinking}")

                                        elif item_type == "image":
                                            text_parts.append("[图片]")

                                content = "\n\n".join(text_parts)

                            # Skip empty and warmup messages
                            if content and content != "Warmup":
                                messages.append({
                                    "role": msg_type,
                                    "content": content,
                                    "timestamp": data.get("timestamp")
                                })
                    except json.JSONDecodeError:
                        continue
        except Exception:
            pass

        return messages

    def delete_session(self, cli_type: str, project_name: str, session_id: str) -> bool:
        """Delete a session file."""
        projects_dir = self._get_projects_dir(cli_type)
        session_file = projects_dir / project_name / f"{session_id}.jsonl"

        if session_file.exists():
            session_file.unlink()
            return True
        return False

    def delete_project(self, cli_type: str, project_name: str) -> bool:
        """Delete a project directory."""
        projects_dir = self._get_projects_dir(cli_type)
        project_dir = projects_dir / project_name

        if project_dir.exists():
            shutil.rmtree(project_dir)
            return True
        return False
