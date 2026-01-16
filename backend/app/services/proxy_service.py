from fastapi import Request, Response, HTTPException
from fastapi.responses import StreamingResponse
from sqlalchemy.ext.asyncio import AsyncSession
import httpx
import json
import time
import logging
import asyncio
import fnmatch
import re
from typing import Optional, AsyncIterator, Tuple
from urllib.parse import quote

from app.services.routing_service import RoutingService
from app.services.provider_service import ProviderService
from app.services.stats_service import StatsService
from app.services.log_service import LogService
from app.models.models import Provider, TimeoutSettings, GatewaySettings
from sqlalchemy import select

logger = logging.getLogger(__name__)


def _truncate_body(body: bytes, max_len: int = 2000) -> str:
    """Truncate body for logging."""
    try:
        text = body.decode("utf-8")
        if len(text) <= max_len:
            return text
        return text[:max_len] + f"... [truncated, total {len(text)} chars]"
    except:
        return f"[binary data, {len(body)} bytes]"


# Headers to filter out when forwarding
FILTERED_HEADERS = {
    "host", "connection", "keep-alive", "transfer-encoding",
    "te", "trailer", "upgrade", "content-length"
}

# Shared HTTP client for connection pooling
_http_client: Optional[httpx.AsyncClient] = None


async def get_http_client() -> httpx.AsyncClient:
    global _http_client
    if _http_client is None:
        _http_client = httpx.AsyncClient(
            timeout=httpx.Timeout(connect=10.0, read=None, write=30.0, pool=10.0),
            limits=httpx.Limits(max_connections=100, max_keepalive_connections=20)
        )
    return _http_client


class ProxyService:
    def __init__(self, db: AsyncSession, routing_service: RoutingService):
        self.db = db
        self.routing_service = routing_service
        self.provider_service = ProviderService(db)
        self.stats_service = StatsService(db)
        self.log_service = LogService(db)

    def _apply_model_mapping(self, provider: Provider, body: bytes) -> Tuple[bytes, Optional[str], Optional[str]]:
        """Apply model mapping to request body. Returns (new_body, original_model, final_model).

        Supports wildcard matching with * (case-insensitive).
        """
        if not body:
            return body, None, None

        try:
            data = json.loads(body)
            if "model" not in data:
                return body, None, None

            original_model = data["model"]

            if not provider.model_maps:
                return body, None, original_model

            for mm in provider.model_maps:
                if mm.enabled and fnmatch.fnmatch(original_model.lower(), mm.source_model.lower()):
                    data["model"] = mm.target_model
                    return json.dumps(data, ensure_ascii=False).encode("utf-8"), original_model, mm.target_model

            return body, None, original_model
        except (json.JSONDecodeError, UnicodeDecodeError):
            return body, None, None

    def _apply_gemini_url_model_mapping(self, provider: Provider, path: str) -> Tuple[str, Optional[str], Optional[str]]:
        """Apply model mapping to Gemini URL path. Returns (new_path, original_model, final_model).

        Gemini URL format: /v1beta/models/{model}:{action}
        """
        # Match Gemini model path pattern
        match = re.match(r'^(v1beta/models/)([^:]+)(:.+)$', path)
        if not match:
            return path, None, None

        prefix, model_name, action = match.groups()

        if not provider.model_maps:
            return path, None, model_name

        for mm in provider.model_maps:
            if mm.enabled and fnmatch.fnmatch(model_name.lower(), mm.source_model.lower()):
                new_path = f"{prefix}{mm.target_model}{action}"
                return new_path, model_name, mm.target_model

        return path, None, model_name

    async def forward_request(self, request: Request, path: str) -> Response:
        """Forward request to upstream provider."""
        start_time = time.time()

        # Detect CLI type from request
        cli_type = self._detect_cli_type(request, path)

        # Select provider
        provider = await self.routing_service.select_provider(cli_type)
        if not provider:
            logger.warning(f"No available provider for cli_type={cli_type}, all providers may be blacklisted")
            # Log the rejected request
            try:
                body = await request.body()
                client_headers = dict(request.headers)
                await self.log_service.create_request_log(
                    cli_type=cli_type,
                    provider_name="[NO_PROVIDER]",
                    client_method=request.method,
                    client_path=path + (f"?{request.url.query}" if request.url.query else ""),
                    client_headers=client_headers,
                    client_body=body.decode("utf-8", errors="replace"),
                    forward_url="",
                    forward_headers={},
                    forward_body="",
                    created_at=int(start_time),
                    success=False,
                    status_code=503,
                    elapsed_ms=int((time.time() - start_time) * 1000),
                    error_message=f"No available provider for {cli_type}"
                )
            except Exception as e:
                logger.error(f"Failed to log rejected request: {e}")
            raise HTTPException(status_code=503, detail=f"No available provider for {cli_type}")

        # Get settings
        timeouts = await self._get_timeout_settings()
        debug_log = await self._get_debug_log()

        # Save original path for logging
        client_path = path

        # Apply model mapping based on CLI type
        original_model = None
        final_model = None
        if cli_type == "gemini":
            # Gemini: model is in URL path
            path, original_model, final_model = self._apply_gemini_url_model_mapping(provider, path)

        # Build upstream URL
        upstream_url = f"{provider.base_url.rstrip('/')}/{path}"
        if request.url.query:
            upstream_url += f"?{request.url.query}"

        # Prepare headers (filter hop-by-hop headers)
        client_headers = dict(request.headers)
        headers = {
            k: v for k, v in request.headers.items()
            if k.lower() not in FILTERED_HEADERS
        }

        # Set auth header based on CLI type
        if cli_type == "gemini":
            # Gemini uses x-goog-api-key header
            headers.pop("authorization", None)
            headers["x-goog-api-key"] = provider.api_key
        else:
            # Claude/Codex use Authorization Bearer
            headers["authorization"] = f"Bearer {provider.api_key}"

        # Get request body
        body = await request.body()
        body_str = body.decode("utf-8", errors="replace")

        # Apply model mapping for non-Gemini (body-based)
        forward_body = body
        if cli_type != "gemini":
            forward_body, body_original_model, body_final_model = self._apply_model_mapping(provider, body)
            if body_original_model:
                original_model = body_original_model
            if body_final_model:
                final_model = body_final_model
        forward_body_str = forward_body.decode("utf-8", errors="replace")

        # Check if streaming
        is_stream = self._is_streaming_request(body, path, cli_type)

        # Build log context
        log_ctx = {
            "cli_type": cli_type,
            "provider_name": provider.name,
            "client_method": request.method,
            "client_path": client_path + (f"?{request.url.query}" if request.url.query else ""),
            "client_headers": client_headers,
            "client_body": body_str,
            "forward_url": upstream_url,
            "forward_headers": headers,
            "forward_body": forward_body_str,
            "created_at": int(start_time),
            "model_id": final_model,
        }

        # Debug log: client request + forwarding request
        if debug_log:
            client_ip = request.client.host if request.client else "unknown"
            model_info = f"\n  Model Mapping: {original_model} -> (mapped)" if original_model else ""
            logger.info(
                f"\n{'='*60}\n"
                f"[DEBUG] === CLIENT REQUEST ===\n"
                f"  Client IP: {client_ip}\n"
                f"  Method: {request.method}\n"
                f"  Path: {client_path}\n"
                f"  Query: {request.url.query}\n"
                f"  Headers: {json.dumps(dict(request.headers), indent=2, ensure_ascii=False)}\n"
                f"  Body: {_truncate_body(body)}\n"
                f"[DEBUG] === FORWARD REQUEST ===\n"
                f"  Provider: {provider.name}{model_info}\n"
                f"  Upstream URL: {upstream_url}\n"
                f"  Headers: {json.dumps(headers, indent=2, ensure_ascii=False)}\n"
                f"  Body: {_truncate_body(forward_body)}\n"
                f"  Stream: {is_stream}\n"
                f"{'='*60}"
            )

        try:
            if is_stream:
                return await self._forward_streaming(
                    provider, upstream_url, request.method, headers, forward_body,
                    timeouts, cli_type, start_time, debug_log, log_ctx
                )
            else:
                return await self._forward_non_streaming(
                    provider, upstream_url, request.method, headers, forward_body,
                    timeouts, cli_type, start_time, debug_log, log_ctx
                )
        except HTTPException:
            raise
        except Exception as e:
            elapsed = int((time.time() - start_time) * 1000)
            logger.error(f"Proxy error for provider {provider.name}: {e}")
            if debug_log:
                logger.info(
                    f"\n[DEBUG] === ERROR ===\n"
                    f"  Provider: {provider.name}\n"
                    f"  Error: {e}\n"
                    f"  Elapsed: {elapsed}ms\n"
                )
            await self.provider_service.record_failure(provider.id)
            await self.stats_service.record_request(provider.id, cli_type, False, 0, 0)
            # Record error log
            if debug_log:
                try:
                    await self.log_service.create_request_log(
                        **log_ctx,
                        success=False,
                        status_code=502,
                        elapsed_ms=elapsed,
                        error_message=str(e)
                    )
                except:
                    pass
            raise HTTPException(status_code=502, detail="Upstream request failed")

    async def _forward_streaming(
        self, provider: Provider, url: str, method: str,
        headers: dict, body: bytes, timeouts: TimeoutSettings,
        cli_type: str, start_time: float, debug_log: bool, log_ctx: dict
    ) -> StreamingResponse:
        """Forward streaming request."""
        client = await get_http_client()

        # 先发起请求获取响应头和状态码
        req = client.build_request(method, url, headers=headers, content=body)
        response = await client.send(req, stream=True)

        if debug_log:
            logger.info(
                f"\n[DEBUG] === PROVIDER RESPONSE (streaming) ===\n"
                f"  Status: {response.status_code}\n"
                f"  Headers: {json.dumps(dict(response.headers), indent=2, ensure_ascii=False)}"
            )

        # 错误响应直接返回
        if response.status_code >= 400:
            error_body = await response.aread()
            await response.aclose()
            elapsed = int((time.time() - start_time) * 1000)
            if debug_log:
                logger.info(f"\n[DEBUG] === ERROR RESPONSE ===\n  Body: {_truncate_body(error_body)}\n  Elapsed: {elapsed}ms\n")
            await self.provider_service.record_failure(provider.id)
            await self.stats_service.record_request(provider.id, cli_type, False, 0, 0)
            # Record error log
            if debug_log:
                try:
                    await self.log_service.create_request_log(
                        **log_ctx,
                        success=False,
                        status_code=response.status_code,
                        elapsed_ms=elapsed,
                        provider_status=response.status_code,
                        provider_headers=dict(response.headers),
                        provider_body=error_body.decode("utf-8", errors="replace"),
                        response_status=response.status_code,
                        response_headers=dict(response.headers),
                        response_body=error_body.decode("utf-8", errors="replace"),
                    )
                except:
                    pass
            return Response(content=error_body, status_code=response.status_code, media_type=response.headers.get("content-type"))

        first_byte_time: Optional[float] = None
        total_bytes = 0
        collected_chunks: list[bytes] = []
        usage = {"input": 0, "output": 0}

        # 透传上游响应头（过滤 hop-by-hop 头）
        resp_headers = {
            k: v for k, v in response.headers.items()
            if k.lower() not in FILTERED_HEADERS and k.lower() != "content-encoding"
        }
        # HTTP 头只支持 ASCII，中文名需要 URL 编码
        resp_headers["X-CCG-Provider"] = quote(provider.name, safe="")

        async def stream_generator() -> AsyncIterator[bytes]:
            nonlocal first_byte_time, total_bytes, collected_chunks, usage
            first_byte_received = False
            success = False
            error_msg = None

            try:
                aiter = response.aiter_bytes()
                while True:
                    timeout_val = timeouts.stream_first_byte_timeout if not first_byte_received else timeouts.stream_idle_timeout
                    try:
                        chunk = await asyncio.wait_for(aiter.__anext__(), timeout=timeout_val)
                    except StopAsyncIteration:
                        success = True
                        break
                    except asyncio.TimeoutError:
                        timeout_type = "First byte" if not first_byte_received else "Idle"
                        error_msg = f"{timeout_type} timeout"
                        logger.warning(f"{timeout_type} timeout for provider {provider.name}")
                        yield f'event: error\ndata: {{"type":"timeout","message":"{timeout_type} timeout"}}\n\n'.encode()
                        break

                    if not first_byte_received:
                        first_byte_received = True
                        first_byte_time = time.time()

                    total_bytes += len(chunk)
                    collected_chunks.append(chunk)
                    self._parse_sse_usage(chunk, cli_type, usage)
                    yield chunk

            except httpx.TimeoutException:
                error_msg = "connection timeout"
                logger.error(f"Timeout for provider {provider.name}")
                yield b'event: error\ndata: {"type":"timeout","message":"connection timeout"}\n\n'
            except Exception as e:
                error_msg = str(e)
                logger.error(f"Streaming error for provider {provider.name}: {e}")
                yield f'event: error\ndata: {{"type":"error","message":"{str(e)}"}}\n\n'.encode()
            finally:
                await response.aclose()
                elapsed = int((time.time() - start_time) * 1000)
                # Re-parse usage from complete response (chunks may have been split)
                if collected_chunks and (usage["input"] == 0 and usage["output"] == 0):
                    full_response = b"".join(collected_chunks)
                    self._parse_sse_usage(full_response, cli_type, usage)
                if success:
                    await self.provider_service.record_success(provider.id)
                    await self.stats_service.record_request(provider.id, cli_type, True, usage["input"], usage["output"])
                else:
                    await self.provider_service.record_failure(provider.id)
                    await self.stats_service.record_request(provider.id, cli_type, False, 0, 0)

                if debug_log:
                    ttfb = (first_byte_time - start_time) * 1000 if first_byte_time else 0
                    logger.info(
                        f"\n[DEBUG] === FORWARD RESULT (streaming) ===\n"
                        f"  Provider: {provider.name}\n"
                        f"  Success: {success}\n"
                        f"  Input Tokens: {usage['input']}\n"
                        f"  Output Tokens: {usage['output']}\n"
                        f"  Total Bytes: {total_bytes}\n"
                        f"  TTFB: {ttfb:.2f}ms\n"
                        f"  Total Elapsed: {elapsed}ms\n"
                    )
                    # Record streaming log
                    try:
                        response_body = b"".join(collected_chunks).decode("utf-8", errors="replace")
                        await self.log_service.create_request_log(
                            **log_ctx,
                            success=success,
                            status_code=response.status_code,
                            elapsed_ms=elapsed,
                            input_tokens=usage["input"],
                            output_tokens=usage["output"],
                            provider_status=response.status_code,
                            provider_headers=dict(response.headers),
                            provider_body=f"[streaming] {total_bytes} bytes",
                            response_status=response.status_code,
                            response_headers=dict(resp_headers),
                            response_body=response_body if len(response_body) < 100000 else f"[streaming] {total_bytes} bytes",
                            error_message=error_msg
                        )
                    except:
                        pass

        return StreamingResponse(
            stream_generator(),
            status_code=response.status_code,
            media_type=response.headers.get("content-type", "text/event-stream"),
            headers=resp_headers
        )

    async def _forward_non_streaming(
        self, provider: Provider, url: str, method: str,
        headers: dict, body: bytes, timeouts: TimeoutSettings,
        cli_type: str, start_time: float, debug_log: bool, log_ctx: dict
    ) -> Response:
        """Forward non-streaming request."""
        client = await get_http_client()
        timeout = httpx.Timeout(connect=10.0, read=timeouts.non_stream_timeout, write=30.0, pool=10.0)

        try:
            response = await client.request(method, url, headers=headers, content=body, timeout=timeout)
            elapsed = int((time.time() - start_time) * 1000)
            response_body = response.content.decode("utf-8", errors="replace")

            # Parse token usage
            usage = {"input": 0, "output": 0}
            self._parse_sse_usage(response.content, cli_type, usage)

            if debug_log:
                logger.info(
                    f"\n[DEBUG] === PROVIDER RESPONSE ===\n"
                    f"  Status: {response.status_code}\n"
                    f"  Headers: {json.dumps(dict(response.headers), indent=2, ensure_ascii=False)}\n"
                    f"  Body: {_truncate_body(response.content)}\n"
                    f"[DEBUG] === FORWARD RESULT ===\n"
                    f"  Provider: {provider.name}\n"
                    f"  Status: {response.status_code}\n"
                    f"  Input Tokens: {usage['input']}\n"
                    f"  Output Tokens: {usage['output']}\n"
                    f"  Response Size: {len(response.content)} bytes\n"
                    f"  Elapsed: {elapsed}ms\n"
                )

            success = response.status_code < 400
            if success:
                await self.provider_service.record_success(provider.id)
                await self.stats_service.record_request(provider.id, cli_type, True, usage["input"], usage["output"])
            else:
                await self.provider_service.record_failure(provider.id)
                await self.stats_service.record_request(provider.id, cli_type, False, 0, 0)

            # Record log
            if debug_log:
                try:
                    await self.log_service.create_request_log(
                        **log_ctx,
                        success=success,
                        status_code=response.status_code,
                        elapsed_ms=elapsed,
                        input_tokens=usage["input"],
                        output_tokens=usage["output"],
                        provider_status=response.status_code,
                        provider_headers=dict(response.headers),
                        provider_body=response_body,
                        response_status=response.status_code,
                        response_headers=dict(response.headers),
                        response_body=response_body,
                    )
                except:
                    pass

            # 过滤 hop-by-hop 头和 content-encoding（httpx 已自动解压）
            resp_headers = {
                k: v for k, v in response.headers.items()
                if k.lower() not in FILTERED_HEADERS and k.lower() != "content-encoding"
            }
            resp_headers["X-CCG-Provider"] = quote(provider.name, safe="")

            return Response(
                content=response.content,
                status_code=response.status_code,
                headers=resp_headers,
                media_type=response.headers.get("content-type")
            )

        except httpx.TimeoutException:
            elapsed = int((time.time() - start_time) * 1000)
            if debug_log:
                logger.info(f"\n[DEBUG] Non-streaming timeout after {elapsed}ms\n")
                try:
                    await self.log_service.create_request_log(
                        **log_ctx,
                        success=False,
                        status_code=504,
                        elapsed_ms=elapsed,
                        error_message="Upstream timeout"
                    )
                except:
                    pass
            await self.provider_service.record_failure(provider.id)
            await self.stats_service.record_request(provider.id, cli_type, False, 0, 0)
            raise HTTPException(status_code=504, detail="Upstream timeout")

    def _detect_cli_type(self, request: Request, path: str) -> str:
        """Detect CLI type from User-Agent."""
        user_agent = request.headers.get("user-agent", "").lower()

        if "codex" in user_agent:
            return "codex"
        if "claude" in user_agent:
            return "claude_code"
        if "gemini" in user_agent:
            return "gemini"

        logger.debug(f"Unknown user-agent, defaulting to claude_code: {user_agent}")
        return "claude_code"

    def _is_streaming_request(self, body: bytes, path: str, cli_type: str) -> bool:
        """Check if request is for streaming."""
        # Gemini: streaming is determined by URL path, not body
        if cli_type == "gemini":
            return ":streamGenerateContent" in path

        try:
            data = json.loads(body)
            return data.get("stream", False)
        except:
            return False

    def _parse_sse_usage(self, chunk: bytes, cli_type: str, usage: dict) -> None:
        """Parse SSE chunk or JSON response for token usage."""
        try:
            text = chunk.decode("utf-8", errors="replace")
            data_list = []

            # Try SSE format first
            for line in text.split("\n"):
                if line.startswith("data: "):
                    data_str = line[6:].strip()
                    if data_str and data_str != "[DONE]":
                        try:
                            data_list.append(json.loads(data_str))
                        except:
                            pass

            # If no SSE data found, try parsing as plain JSON
            if not data_list:
                try:
                    data_list.append(json.loads(text.strip()))
                except:
                    pass

            for data in data_list:
                if cli_type == "claude_code":
                    # message_start: message.usage has input_tokens
                    msg_usage = data.get("message", {}).get("usage", {})
                    if "input_tokens" in msg_usage:
                        usage["input"] = msg_usage["input_tokens"]
                    if "output_tokens" in msg_usage:
                        usage["output"] = msg_usage["output_tokens"]
                    # message_delta: usage has output_tokens only
                    direct_usage = data.get("usage", {})
                    if "input_tokens" in direct_usage:
                        usage["input"] = direct_usage["input_tokens"]
                    if "output_tokens" in direct_usage:
                        usage["output"] = direct_usage["output_tokens"]
                elif cli_type == "codex":
                    # Only response.completed event contains usage data
                    if data.get("type") == "response.completed":
                        resp_usage = data.get("response", {}).get("usage", {})
                        if "input_tokens" in resp_usage:
                            usage["input"] = resp_usage["input_tokens"]
                        if "output_tokens" in resp_usage:
                            usage["output"] = resp_usage["output_tokens"]
                elif cli_type == "gemini":
                    meta = data.get("usageMetadata", {})
                    if "promptTokenCount" in meta:
                        usage["input"] = meta["promptTokenCount"]
                    output_tokens = meta.get("candidatesTokenCount", 0) + meta.get("thoughtsTokenCount", 0)
                    if output_tokens > 0:
                        usage["output"] = output_tokens
        except:
            pass

    async def _get_timeout_settings(self) -> TimeoutSettings:
        """Get timeout settings from database."""
        result = await self.db.execute(select(TimeoutSettings).where(TimeoutSettings.id == 1))
        settings = result.scalar_one_or_none()
        if not settings:
            return TimeoutSettings(
                stream_first_byte_timeout=30,
                stream_idle_timeout=60,
                non_stream_timeout=120
            )
        return settings

    async def _get_debug_log(self) -> bool:
        """Get debug_log setting from database."""
        result = await self.db.execute(select(GatewaySettings).where(GatewaySettings.id == 1))
        settings = result.scalar_one_or_none()
        return bool(settings.debug_log) if settings else False
