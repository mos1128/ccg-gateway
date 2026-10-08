//! 词条域：代理层（HTTP 错误响应体、SSE 错误帧、请求日志错误码兜底文案）

use crate::i18n::Lang;

pub fn text(lang: Lang, key: &str) -> Option<&'static str> {
    let text = match key {
        // 无可用服务商 / 全部失败
        "err.proxy.no_provider_passthrough" => match lang {
            Lang::Zh => "没有可用的服务商，无法透传该请求",
            Lang::En => "No provider available to pass through this request",
        },
        "err.proxy.no_provider" => match lang {
            Lang::Zh => "没有可用的服务商",
            Lang::En => "No provider available",
        },
        "err.proxy.all_providers_failed" => match lang {
            Lang::Zh => "所有服务商均未能成功响应",
            Lang::En => "All providers failed to respond successfully",
        },

        // 上游请求失败 / 超时
        "err.proxy.upstream_no_response" => match lang {
            Lang::Zh => "上游请求失败，未收到响应",
            Lang::En => "Upstream request failed; no response received",
        },
        "err.proxy.upstream_request_failed" => match lang {
            Lang::Zh => "上游请求失败: {}",
            Lang::En => "Upstream request failed: {}",
        },
        "err.proxy.passthrough_timeout" => match lang {
            Lang::Zh => "透传请求超时",
            Lang::En => "Passthrough request timed out",
        },
        "err.proxy.request_timeout" => match lang {
            Lang::Zh => "请求超时",
            Lang::En => "Request timeout",
        },
        "err.proxy.first_byte_timeout" => match lang {
            Lang::Zh => "首字节超时",
            Lang::En => "First-byte timeout",
        },
        "err.proxy.body_read_timeout" => match lang {
            Lang::Zh => "响应体读取超时",
            Lang::En => "Response body read timed out",
        },

        // 响应体读取
        "err.proxy.read_upstream_body_failed" => match lang {
            Lang::Zh => "读取上游响应体失败: {}",
            Lang::En => "Failed to read upstream response body: {}",
        },
        "err.proxy.read_upstream_body_failed_generic" => match lang {
            Lang::Zh => "读取上游响应体失败",
            Lang::En => "Failed to read upstream response body",
        },
        "err.proxy.read_upstream_error_body_failed" => match lang {
            Lang::Zh => "上游错误响应体读取失败",
            Lang::En => "Failed to read the upstream error response body",
        },
        "err.proxy.read_upstream_error_response_failed" => match lang {
            Lang::Zh => "读取上游错误响应失败: {}",
            Lang::En => "Failed to read the upstream error response: {}",
        },
        "err.proxy.read_upstream_error_response_timeout" => match lang {
            Lang::Zh => "读取上游错误响应超时",
            Lang::En => "Timed out reading the upstream error response",
        },

        // 透传 / 转换
        "err.proxy.passthrough_build_failed" => match lang {
            Lang::Zh => "透传响应构造失败",
            Lang::En => "Failed to build passthrough response",
        },
        "err.proxy.compressed_stream" => match lang {
            Lang::Zh => "上游压缩了流式响应，无法转换协议",
            Lang::En => "Upstream compressed the streaming response; protocol conversion is not possible",
        },
        "err.proxy.translate_to_client_failed" => match lang {
            Lang::Zh => "上游响应无法转成客户端协议",
            Lang::En => "Upstream response cannot be converted to the client protocol",
        },
        "err.proxy.translate_skipped" => match lang {
            Lang::Zh => "请求体无法转成上游协议，跳过该服务商",
            Lang::En => "Request body cannot be converted to the upstream protocol; skipping this provider",
        },
        "err.proxy.retry_after_strip" => match lang {
            Lang::Zh => "上游拒绝历史思考块，剥掉私有载荷回放后重试",
            Lang::En => "Upstream rejected historical thinking blocks; stripping private payloads and retrying",
        },

        // 流式错误
        "err.proxy.stream_interrupted" => match lang {
            Lang::Zh => "上游流中断，未完成",
            Lang::En => "Upstream stream interrupted before completion",
        },
        "err.proxy.upstream_response_error" => match lang {
            Lang::Zh => "上游响应错误: {}",
            Lang::En => "Upstream response error: {}",
        },
        "err.proxy.empty_stream" => match lang {
            Lang::Zh => "上游返回了空流",
            Lang::En => "Upstream returned an empty stream",
        },
        "err.proxy.stream_error" => match lang {
            Lang::Zh => "上游流错误: {}",
            Lang::En => "Upstream stream error: {}",
        },
        "err.proxy.stream_protocol_error" => match lang {
            Lang::Zh => "上游流协议错误: {}",
            Lang::En => "Upstream stream protocol error: {}",
        },
        "err.proxy.stream_idle_timeout" => match lang {
            Lang::Zh => "流空闲超时",
            Lang::En => "Stream idle timeout",
        },
        "err.proxy.stream_ended_before_event" => match lang {
            Lang::Zh => "上游流在有效事件前结束",
            Lang::En => "Upstream stream ended before a valid event",
        },
        "err.proxy.stream_ended_before_completion" => match lang {
            Lang::Zh => "上游流在完成事件前结束",
            Lang::En => "Upstream stream ended before the completion event",
        },
        "err.proxy.invalid_json_response" => match lang {
            Lang::Zh => "上游返回了无效的 JSON 响应",
            Lang::En => "Upstream returned an invalid JSON response",
        },
        "err.proxy.upstream_http_error" => match lang {
            Lang::Zh => "上游返回了 HTTP 错误",
            Lang::En => "Upstream returned an HTTP error",
        },
        "err.proxy.upstream_http_status" => match lang {
            Lang::Zh => "上游返回 {}",
            Lang::En => "Upstream returned {}",
        },

        // 客户端断开 / 网关重启
        "err.proxy.client_closed" => match lang {
            Lang::Zh => "客户端在完成前断开了连接",
            Lang::En => "Client disconnected before completion",
        },
        "err.log.gateway_restarted" => match lang {
            Lang::Zh => "网关在请求完成前停止",
            Lang::En => "Gateway stopped before request completed",
        },

        _ => return None,
    };
    Some(text)
}
