# 协议互转：坑点与未解决问题

记录 `src-tauri/src/services/translate/` 三方互转（Anthropic Messages ↔ OpenAI Chat Completions ↔ OpenAI Responses）里**已知但没修**和**修不了**的问题。目的是让后来的人不必重新踩一遍，也不要去追已经查清是误报的线索。

Gemini 不参与互转（`Format::from_protocol` 对 `GeminiGenerateContent` 返回 `None`），只走直通，本文不涉及。

## 怎么读这份文档

每条按同一格式写：

- **位置**：文件 + 函数
- **现状**：代码现在怎么做的
- **后果**：客户端/上游会看到什么
- **实测规模**：来自 `~/.ccg-gateway/request-bodies/` 的真实语料。那里每个请求有三份：`*-client.body`（客户端发来的）、`*-forward.body`（我们转发出去的）、`*-provider.body`（**上游真正回给我们的响应**，21795 份）。所以「请求长什么样」和「上游的流长什么样」都是实测的，见第五节
- **为什么不改**：设计选择，还是确实无解
- **怎么确认碰到了**：线上出问题时怎么定位到这一条

分级：

- **无解**：协议之间没有等价物，任何实现都会丢信息
- **能修但没修**：有解法，但代价大于收益，或者要改的地方超出当前范围
- **踩过的坑**：已经修好，写在这里是为了别再改回去

## 一、无解：协议之间没有等价物

### A-1 推理过程跨协议不可迁移

**位置** `ir.rs` 的 `Opaque` 信封、`anthropic.rs::build_request` 的 thinking 闸门、`mod.rs::strip_thinking`。

**现状** 三家都有「模型的内部推理」这个概念，但载荷互不通用：Anthropic 的 `thinking` 块带 `signature`（服务端签名，跨账号都验不过），Responses 的 `reasoning` 项带 `encrypted_content`（上游私钥加密），Chat 只有 `reasoning_content` 纯文本。我们把这些私有载荷包进 `ccg-opaque-v1:` 信封，**format 和 key（服务商 id）同时匹配**才回放；不匹配就整块丢掉。

**后果** 一旦发生渠道轮转或跨协议转换，历史里的推理块全部失效，只能丢。丢掉的直接影响是：Anthropic 上游看到「assistant 消息以 `tool_use` 开头但没有前置 thinking 块」会 400；模型也失去了上一轮的思考上下文，多轮工具调用的连贯性变差。

**为什么无解** 签名和密文都是上游服务端生成的，我们没有密钥，也没有等价的明文可以替代。唯一的选择是丢，以及丢干净。

**兜底做法** 两层：

1. `build_request` 的 thinking 闸门末尾有 `thinking_replay_ok`，只检查**最后一条** assistant 消息——如果它没有 `tool_use` 就直接返回 `true`（没有工具调用就不存在「必须以 thinking 开头」的约束）。有 `tool_use` 而签名回放不成立时，整个请求就不开 thinking。
2. `handlers.rs:626` 的事后整流：4xx 时把错误体读出来喂给 `rejects_thinking`（9 组关键词，见 `handlers.rs:767`），命中就把 `replay_key` 换成 `NO_REPLAY`（`"\0"`，永不匹配任何服务商 id），剥掉全部私有载荷后**在同一个渠道**重试一次。整流每个渠道只做一次（`rectified` 标记）。

**怎么确认碰到了** 日志里 `上游拒绝历史思考块，剥掉私有载荷回放后重试`。如果上游的报错措辞不在那 9 组关键词里，就会卡在 400 走正常轮转——这时要做的是往 `SCENARIOS` 加一组关键词，不是放宽成「所有 4xx 都整流」（那样普通参数错误也会被当成整流机会，白重试一轮）。

### A-2 Codex 的 reasoning 项绝大多数只有密文没有摘要，转出去就是空

**位置** `openai_responses.rs::parse_request` 处理 `type: "reasoning"` 的分支。

**现状** Responses 的 reasoning 项有两个字段：`summary`（人类可读摘要）和 `encrypted_content`（密文）。我们把 `summary` 转成 IR 的思考文本，把 `encrypted_content` 包进信封。**`summary` 为空时整项被丢弃**——没有任何可转成文本的内容。

**实测规模** 497444 个 reasoning 项里 484998 个（97.5%）没有 summary。Codex 默认不请求摘要。

**后果** Codex 客户端 → 非 Responses 上游时，历史里几乎所有推理都消失。对模型而言等于「上一轮我什么都没想」。

**为什么无解** 同 A-1，密文解不开。理论上可以让网关自己往上游请求里塞 `reasoning.summary: "auto"` 强行要摘要，但那会改变客户端的计费和延迟特征，属于替用户做决定，没做。

**怎么确认碰到了** Codex 跨协议转换时多轮工具调用质量明显下降，但没有任何报错。

### A-3 Responses 的 `item_reference` 指向上游服务端存的会话，我们没有

**位置** `openai_responses.rs::parse_request`，未识别的 item 类型走 `_ => {}`。

**现状** Responses 允许客户端只发 `{type: "item_reference", id: "..."}` 引用上游已存的历史项（配合 `store: true` 用）。我们没有那个存储，直接丢。

**后果** 用了服务端会话存储的客户端跨协议转换时，被引用的历史整段消失。

**为什么无解** 引用的内容在上游服务器上，跨服务商更不存在。要支持就得网关自己做一份会话存储并代理 `store` 语义，那是另一个功能，不是转换层的事。

**实测规模** 当前语料里没有出现。Codex 走 `store: false`。

### A-4 Codex 的自定义工具语法（lark grammar）被压成 `{input: string}`

**位置** `openai_responses.rs::custom_schema()`。

**现状** Responses 的 `type: "custom"` 工具可以带 `format: {type: "grammar", syntax: "lark", definition: "..."}`，让模型按上下文无关文法输出。Anthropic 和 Chat 都只有 JSON Schema。我们把它降级成 `{"type":"object","properties":{"input":{"type":"string"}}}`。

**实测规模** 169 个请求带 `output_config.format`（Codex 的 `apply_patch` 之类）。

**后果** 模型不再受文法约束，输出格式对不上客户端的解析器时会失败。

**为什么无解** JSON Schema 表达不了 CFG。反方向（Anthropic → Responses）不受影响，因为 JSON Schema 是子集。

**注意** `ir.rs::strip_unsupported` 只剥**字符串形式**的 `format: "uri"`，不会误伤这里的对象形式 `format: {type: grammar}`。改那个函数时别把条件放宽成「见到 format 就删」。

### A-5 思考预算与 effort 之间是有损映射

**位置** `ir.rs` 的 `Effort`、`anthropic.rs::budget_from_effort`、`openai_chat.rs::chat_effort`。

**现状** 三家的思考旋钮都是**档位词**，而且 Anthropic 与 Responses 的取值集合是同一套（`low` / `medium` / `high` / `xhigh` / `max`），所以这两边一律**原样透传**，不换算。只有 Chat 的 `reasoning_effort` 收窄到 low / medium / high，到它那儿才封顶。

| | 开关 | 强度 |
| --- | --- | --- |
| Anthropic | `thinking.type`：`adaptive` / `disabled` | `output_config.effort` |
| Responses | —（给了 `reasoning` 就算开） | `reasoning.effort` |
| Chat | — | `reasoning_effort`（只三档） |

**Anthropic 侧两个旋钮独立，要写就一起写** `thinking.type` 管开不开，`output_config.effort` 管用多大力，不是同一件事——实测 284 条真实请求同时发 `thinking: disabled` + `effort: high`。所以 `anthropic.rs::build_request` 要么写 `{"type": "adaptive"}` + `effort`，要么写 `{"type": "disabled"}`；**关的时候必须显式写 `disabled`，不能省掉 `thinking` 字段**——省掉等于交给上游默认，现在的模型默认是开的，而历史里又没有可验签的思考块，照样 400。

**实测规模** 客户端发过的全部取值：`max` 6625、`xhigh` 6595、`high` 4716、`none` 2、`medium` 1。词形式在真实 Anthropic 端点上是实测可用的：我们发出去的 `effort=high` 4290 次 200、`effort=xhigh` 3521 次 200。

**老写法 `thinking.budget_tokens` 不认** 走互转的只有 Claude Code 和 Codex，两家都只发档位词——实测最近一周 3707 条 Anthropic 请求里 `budget_tokens` **0 条**。多协议客户端（opencode 这类，7 月的语料里发过 4 条）本身就不需要互转，硬要在那上面开互转是使用方式的问题，不为它保留一条猜档位的代码路径。带着 `budget_tokens` 进来时按「没给档位」处理，思考不开。

**`none` / `minimal` 落到 `disabled`** 这两个档是 OpenAI 两家才有的「不要思考」，Anthropic 的 `effort` 里没有对应取值，原样透传就是 400。`anthropic.rs::no_thinking` 把它们映射成 `thinking: disabled`。

**别改回去** 别把档位换算成 `thinking.budget_tokens`。那个版本里 `xhigh` 和 `max` 都变成同一个数（24576），客户端本来的区分在翻译时被抹平；而且预算要受 `max_tokens / 2` 夹紧，Codex 九成请求不带 `max_output_tokens`（9372/9397），夹的其实是网关的兜底值，等于用一个跟客户端无关的数字决定思考量。也别给 Anthropic / Responses 侧加档位白名单——取值集合和客户端是同一套，白名单只会把上游本来支持的档砍掉。`ultracode` 那种客户端侧的复合档（xhigh + workflow）在到达网关之前就已经被客户端展开了，语料里从未出现，不用为它写降解。

### A-6 `context_management` 在 OpenAI 两家没有对等物

**位置** `anthropic.rs::parse_request` 读不到它，`ir.rs::ChatRequest` 也没有承载它的字段。

**现状** Anthropic 客户端会带 `context_management`，让**上游服务端**按规则处理历史。语料里只出现过一种形状：

```json
"context_management": {"edits": [{"type": "clear_thinking_20251015", "keep": "all"}]}
```

**实测规模** 2026-09-03 抽样 587 条客户端请求体，568 条带这个字段（96.8%），Claude Code 几乎每次都发；另两天比例接近。

**后果** 转到 Chat/Responses 时整段丢掉。注意它管的是「服务端怎么处理历史里的思考块」，**不是**「对话太长就砍前面几轮」。丢掉的直接后果是历史里的思考块继续占位置，更容易碰到长度上限，而不是压缩能力整段消失。

**为什么无解** OpenAI 两家没有等价字段。Responses 的 `truncation: "auto"`（丢掉中间的 input 项）是**另一种**行为，硬映射等于替用户改语义。网关自己也不接管这件事：`openai_responses.rs::build_request` 固定写 `store: false`，历史每轮都由我们自己拼全。

**决定** 丢掉，不在转换层假装能转。上下文真满了让别的能力去触发压缩。

## 二、能修但没修

编号有断档是故意的：B-1 已修（见 C-11），B-2 查明不成立（见第四节），其余条目的编号不动，免得旧笔记里的引用错位。

### B-3 会话中间的 `role: system` 被降成 `user`

**位置** `anthropic.rs::parse_request`。

**现状** Anthropic 的 system 只能在顶层 `system` 字段，`messages` 里不允许。Chat/Responses 允许 system/developer 消息出现在会话中间。我们把中间的 system 消息转成 `Role::User`。

**实测规模** 130034 条消息（占 96.2% 的请求）。这是**规模最大的一条**。

**后果** 指令的权重从「系统级」降到「用户级」，模型可能不再优先遵守。

**为什么没修** 三种可选做法都有损：(a) 合并到顶层 system——会改变指令在对话里的时间位置，前面的对话可能已经推翻了它；(b) 丢掉——直接丢指令，更糟；(c) 现在这样降成 user——位置对，权重降。选了 (c)，因为位置错比权重低更难排查。

**同类** `openai_responses.rs::parse_request` 把所有 `developer` 项**上提**合并进顶层 `system`。这条曾被怀疑会打断 Anthropic 的前缀缓存，已查明是误报（见第四节）。

### B-4 无 schema 的服务端内置工具被丢弃

**位置** `openai_responses.rs::push_tool` 的 `_ => return`。

**现状** `web_search`、`file_search`、`computer_use` 这类 Responses 服务端工具没有 `parameters`，转不成 Anthropic/Chat 的 function 定义，整个丢掉。

**实测规模** 112 个请求带这类工具。

**后果** 模型不知道自己有这些能力。如果历史里已经有对应的 `tool_use`，展平逻辑（见 C-1）会把它变成文本，不会 400。

**为什么没修** 目标协议侧没有等价物。Anthropic 有自己的 `web_search_20250305` 之类服务端工具，名字和参数都不一样，硬映射等于猜——猜错了模型调用失败，比没有更糟。

### B-5 `ToolSearch` / `LocalShell` / `Custom` 的响应事件解码器不认

**位置** `openai_responses.rs` 的流式 `Decoder`：`item_added` 只认 `type == "function_call"`，`item_done` 只有 `function_call` / `message` / `reasoning` 三个分支，`response.custom_tool_call_input.delta` 这个事件名压根不在匹配表里。

**现状** Codex 私有的 `custom_tool_call` / `tool_search_call` / `local_shell_call` 从上游流里回来时会被整个丢掉。

**实测规模** 这些形状在语料里非常常见：上游响应里 `custom_tool_call` 7756 次，是普通 `function_call`（754 次）的十倍；`response.custom_tool_call_input.delta` 出现 269 万次，是整批语料里最高频的事件。

**为什么不修：不可达，不是「没出现过」** 上面那 7756 次全部发生在「Codex 客户端 → Codex 上游」，那条路是同协议直通，压根不进 `Decoder`。走转换时我们向上游声明的工具**只有普通函数**（`build_request` 里固定写 `"type": "function"`），上游也就只可能回 `function_call`。上游要回私有形状，前提是请求里声明了私有形状——那不会发生。

**别去补这几个分支** 补了验证不了，而且改动会碰到 `slot` / `open` / `deltas` 那套槽位簿记，引入真 bug 的风险大于收益。这段是防御性代码，按不可达处理。

### B-6 Anthropic 流式 `message_start` 的 usage 硬编码为 0

**位置** `anthropic.rs` 的 `Encoder`。

**现状** `message_start` 事件里的 `usage` 全填 0，真实数字在 `message_delta` 里补。

**实测规模** 真实 Anthropic 系上游 8087 次 `message_start` 里，5469 次（68%）当场就给了 `input_tokens`，所以这个差异是真的存在，不是理论问题。

**后果** 只读 `message_start` 就统计 token 的客户端会得到 0。

**为什么没修** 流式转换时上游的 input tokens 要到最后才知道，`message_start` 必须先发。除了缓冲整个响应（破坏流式）没有别的办法。绝大多数客户端读 `message_delta`；网关自己的用量统计也不受影响——它在转换模式下是从**原始上游字节**按上游协议解析的（`handlers.rs::2139`），不看我们写出去的这份。

### B-7 `Encoder::close` 不清 `slots`

**位置** `anthropic.rs::Encoder::close`。

**现状** 关闭时没清空 `self.slots`。

**后果** 当前调用方每个响应新建一个 Encoder，所以不会出问题。哪天改成复用 Encoder 就会串块。

**为什么没修** 现在不是 bug，改了是为将来的假想用法做准备。记在这里，复用前先清。

### B-8 流式 `ToolStart` 可能带空 id/name

**位置** `openai_chat.rs` 的 `Decoder`。

**现状** 第一次见到某个 tool_call 槽位就发 `ToolStart`，此时 `id`/`name` 可能还没到（有些上游先发 index 再发 id）。

**后果** 下游拿到空名字的工具开始事件。后续 delta 会补上，但已经发出去的事件改不了。

**为什么没修** 要修就得攒到 id 和 name 都齐了再发 `ToolStart`，等于给流式加一层缓冲，首字节延迟变差。权衡后选了先发。

### B-9 Anthropic `Decoder` 忽略预填的 `tool_use.input`

**位置** `anthropic.rs::Decoder` 的 `content_block_start`。

**现状** 假定 `tool_use` 块的 `input` 一开始是空对象，参数全靠后续 `input_json_delta` 累积。如果上游在 `content_block_start` 里就给了非空 `input`，这部分被忽略。

**实测规模** 10394 次 `tool_use` 的 `content_block_start` 里，只有 6 次带了非空 `input`，而且值都是空字符串 `""`——没有任何参数会因此丢失。

**后果** 理论上参数丢失；实测无影响。

**为什么没修** 官方 Anthropic 不这么发，兼容端点实测也不这么发。已按不影响处理。

### B-10 `convert_request` 永远返回 `Some`

**位置** `mod.rs::convert_request`。

**现状** 只要两边协议都能映射到 `Format`，就一定返回 `Some`。`handlers.rs:481` 那个 `(Some(_), None)` 分支（转换失败 → 跳过该服务商）实际只在 JSON 解析失败时触发。

**为什么没修** 曾考虑让转换器在「这个请求转不过去」时返回 `None`，后来靠工具展平（C-1）让所有请求都能转过去，就不需要这个契约了。分支留着是为了 JSON 解析失败的兜底，不是死代码。

## 三、踩过的坑（已修，别改回去）

这几条都改过一版又推翻过，写下原因是因为「更自然」的写法恰好是错的。

### C-1 对不上的 `tool_use` / `tool_result` 展平成文本，而不是让请求转换失败

**位置** `anthropic.rs::flatten_tool_blocks`。

**坑** Anthropic 要求 `messages` 里出现的每个 `tool_use` 都能在 `tools` 里找到定义，否则 400 `No such tool`；落单的 `tool_result`（找不到对应调用）同样会被拒。两种触发场景：本轮压根不声明工具（Codex 的上下文压缩请求，`tools` 是空的），以及清单中途变了（MCP 服务器掉线、用户关掉某个工具）而历史里留着已经不存在的调用。

**做法** 先扫一遍收出「对不上的 id」——工具名不在 `tools` 里的调用，以及 id 找不到对应调用的结果——**只展平这些**，转成文本块（工具名 + 参数摘要），配对的另一半跟着一起展平。模型仍然看得到「上一轮调了什么」，请求也能过。

**别改回去** 三条：

1. 别改成「有孤儿就返回 `None` 让请求转换失败」——那样整个渠道被 `skipped`，用户看到的是转换失败而不是一个能用的回答。
2. 别改成「凭空补一个工具定义」——模型会以为自己真能调那个工具。
3. 别改回「`tools` 全空才展平，且一次展平全部」。那个版本漏掉了「清单里还剩别的工具、只有一个对不上」这种情况，而它恰好是最常见的触发方式（MCP 掉线）；这种 400 换渠道也救不了，每家都会拒，会话直接死掉。反过来也别把它写成「有对不上的就全表展平」——那会让本来能用的工具在这一轮集体失效。

**触发前后一致性**（实测 9397 条 Codex 请求）：老逻辑触发 84 条，新逻辑同样是那 84 条，且在这 84 条里展平的块集合完全相同——`tools` 为空时每个调用都对不上，两种写法等价。剩下 9313 条一条都不展平。

### C-2 `finish_reason` 只在 `None | Stop` 时才升级成 `ToolUse`

**位置** `openai_chat.rs::finish_with_tools`。

**坑** 有些兼容端点给了 `tool_calls` 却回 `finish_reason: "stop"`（或者压根不给）。按结束标记决定要不要执行工具的客户端会当成本轮结束，工具不执行，会话空转。

**做法** 有工具块时把 `None` 和 `Stop` 规整成 `ToolUse`。

**别改回去** 曾经写成无条件 `match`，把 `Length` 和 `ContentFilter` 也改成了 `ToolUse` —— 这是错的。工具调用被截断时 Anthropic 自己就回 `stop_reason: "max_tokens"`，客户端必须知道参数 JSON 是残缺的，而不是拿去执行。反方向也不能做：没有工具块时凭空写 `tool_use`，Anthropic 会要求 `content` 里必须有对应的块。

`openai_responses.rs` 那边的 `if self.finish.is_none()` 是对的，别顺手改成一样的形式——Responses 的完成语义不同。

### C-3 `tool_call_id` 超长要哈希，不能截断

**位置** `openai_chat.rs::short_tool_id`。

**坑** Chat 的 `tool_call_id` 上限 40 字符。兼容端点常发 `call-<uuid>-<n>` 形式，43–45 字符，直接透传就 400。

**实测规模** 19.6% 的请求命中。Anthropic 侧 `tool_use.id` 实测最短 29、中位 35、**最长 45** 字符，确实会超。

**做法** 超过 40 就换成 `call_{:016x}`（`DefaultHasher`）。用 `DefaultHasher`（SipHash，进程内带随机种子）没问题，因为一致性只需要在**同一个请求内**成立：`build_user` 里 tool 消息的 id 和 `build_assistant` 里 `tool_calls[].id` 都过同一个函数，同一进程同一请求算出来必然相同。

**别改回去** 别改成截断——不同的长 id 截断后可能撞成同一个，工具结果配错调用。也别为了「跨进程稳定」换成 SHA——没有跨请求持久化的需求。

### C-4 截断检测：三个协议各有各的成功标记，都不许伪造

**位置** Chat `received_finish || saw_done`、Responses `received_complete`、Anthropic `received_stop`。

**坑** 上游流断在中途时，如果转换层补一个正常的结束事件，客户端会把残缺回答当成完整回答。

**实测规模** 这不是罕见情况：8110 条 Anthropic 上游流里 **165 条（2%）没有 `message_stop`**，9064 条 Responses 流里 **43 条**没有 `response.completed` / `incomplete` / `failed`。平均每 50 次请求就用上一次这条防护。

**做法** 三条路径都只在真的收到上游的结束标记时才发结束事件；没收到就让流以错误结束。

**别改回去** 别为了「让客户端不报错」补一个成功结尾。

### C-5 `tool_result` 空内容要填占位符

**位置** `ir.rs::TOOL_NO_OUTPUT`，在 `anthropic.rs`、`openai_responses.rs`（`function_call_output.output`）都用到。

**坑** 工具返回空字符串时，部分上游拒绝空的 tool result。

**做法** 填一个固定占位文本。

### C-6 `Opaque` 回放要 format 和 key 双匹配

**位置** `ir.rs::Opaque::replay`。

**坑** 只匹配 format 的话，A 服务商的签名会被回放给 B 服务商，必然 400（签名是服务端私钥签的）。

**做法** format（来源协议）和 key（服务商 id）都相同才回放。`NO_REPLAY = "\0"` 作为永不匹配的 key 用于整流轮。`placeholder_signature()` 是空 key + null payload 的 Anthropic 信封，用于「需要有个 thinking 块占位但没有真载荷」的场合。

**别改回去** 别放宽成「同协议就回放」。

### C-7 `strip_unsupported` 只剥字符串形式的 `format: "uri"`

**位置** `ir.rs::strip_unsupported`。

**坑** JSON Schema 里的 `format: "uri"` 等关键字部分上游不认。

**实测规模** schema 里的 `format` 取值：字符串形式 `uri` 1741 次、`uuid` 280 次、`uint64` 28 次、`uint32` 14 次、`uint8` 7 次；**对象形式 360 次**。

**做法** 只删字符串形式的。**必须**保持这个限定：对象形式那 360 次里，大部分是普通工具的参数正好叫 `format`（比如某个截图工具的 `format: {type: "string", enum: ["png","jpeg","webp"]}`），另一部分是 Codex 的语法定义 `format: {type: "grammar", ...}`（A-4）。放宽成「见到 format 就删」会把这些参数定义整个删掉，模型就不知道有这个选项了。

**为什么 `uuid` / `uint64` 这些没一起删** 没有证据说它们会被拒。`uri` 进名单是因为 Claude Code 的 WebFetch 带这个、当时观测到被拒；其余取值属于「同一个校验器大概也会拒」的推测。整批语料 75 条 4xx 里没有一条提到 format，所以按「等真被拒了再按错误信息补」处理，不按猜测扩名单。现在 4xx 的上游错误体已经完整记进 `request_logs.error_message`（2026-08-31 起填充率 100%），下次真碰上能直接看到是哪个关键字。

### C-8 `max_tokens` 兜底值可配置且有下限

**位置** `ir.rs::DEFAULT_MAX_TOKENS = 32000`，`handlers.rs:363` 读 `gateway_settings.translate_max_tokens` 并 `.max(1024)`。

**坑** Anthropic 的 `max_tokens` 必填，Chat/Responses 选填。源请求没给时必须编一个。

**实测规模** 这个兜底是常态，不是兜底：9397 条 Codex 请求里 **9372 条压根不带 `max_output_tokens`**（99.7%）。反方向的 Claude Code 请求基本都带（`max_tokens: 64000` 8409 条）。

**做法** 数据库里可配（前端设置页 `translate_max_tokens`），读失败回落到 32000，最后夹一个 1024 下限，防止配成 0 把所有回答掐死。

**它只影响回答长度，不再影响思考量** 早先的版本用 `max_tokens / 2` 夹思考预算，那时这个值配错会连思考一起掐掉；现在思考走档位词（A-5），`max_tokens` 只决定回答上限，用户不填也无所谓。

### C-9 转换路径上的保活必须转出去

**位置** `mod.rs::StreamTranslator::keepalive`、`mod.rs::Encoder::keepalive`。

**坑** 上游真正开口之前常先发保活：OpenAI 系是 SSE 注释 `: PING`，Anthropic 是 `ping` 事件。原来的写法是「编码器还没发过 Start 就把保活丢掉」，怕的是「上游只发垃圾帧」被当成活流。后果是两条路径的超时行为分叉：直通路径上一行 `:` 注释就让 `handlers.rs::looks_like_sse` 判成 `Ready`、首字节计时停表；转换路径什么都不发，`read_stream_prefix` 的 `stream_first_byte_timeout`（网关设置项，默认 30 秒，读不到时代理层回落 60 秒）在上游完全健康的情况下空转到超时。

**实测规模** 真实上游确实这么发：235 条流以 `: ping` 开头，6 条以 `: FIRST_TOKEN_POOL` 开头，还有以三个裸 `:` 开头再接 `response.failed` 的。

**做法** 见到保活就发一帧**客户端协议**的保活。Anthropic 目标在 `message_start` 之后用官方 `ping`，之前（以及 Chat / Responses 目标）用 `KEEPALIVE_COMMENT` 这行 SSE 注释——三家都把 `:` 开头的行当无内容跳过，任何时点发都安全。

**代价（故意接受的）** 保活一发出去，这条渠道就算「已开始」，之后上游只发垃圾也不会再静默换渠道，而是把错误交给客户端。这与直通路径的既有行为一致，是特意对齐的。别为了「还能换渠道」把保活改回丢掉：那等于让转换路径比直通更容易超时。

### C-10 流式请求下的整块 JSON：先判错，且没有内容块不算成功

**位置** `mod.rs::pending_body`、`mod.rs::finish`、`handlers.rs::translated_stream`。

**坑** 兼容端点在 `stream: true` 下回一整块 JSON 很常见（语料 208/3960，其中 139 个是错误体）。判错原来喂的是**整条流的字节**：上游只要先发过一句 `: PING`，拼起来就不是合法 JSON，`response_body_error` 直接返回 None；接着 `finish` 把缓冲里的错误体摊成 `Start + Finish{None}`，Anthropic 编码器写出 `stop_reason: "end_turn"`——客户端看到「空内容 + 正常结束」，网关记成功、不换渠道。

**实测规模** 「先保活再甩整块 JSON 错误体」这个组合在语料里有 4 条，正是它让「拼整条流」的写法失效。另有 502 条上游回的是纯文本 `error code: 502`（压根不是 JSON，走不到判错，靠解码器没收到终止标记来报错）。

**做法** 两层。判错只喂 `pending_body()`，也就是还没被当成 SSE 帧吃掉的那一段，正好是那块 JSON；`finish` 摊开之前再看一眼，一个内容块都没有就发错误事件，不发成功结尾。

**别改回去** 别把判错改回「整条流拼起来解析」。也别为了「让客户端不报错」在没有内容块时补一个正常结尾（同 C-4）。

### C-11 关思考要两个开关一起关，且和有没有剥到块无关

**位置** `anthropic.rs::strip_thinking`、`anthropic.rs::disable_thinking`。

**坑** 两处。(a) 原来 `if !changed { return false }` 排在孤儿 `tool_use` 检查之前，而触发整流的 `"must start with a thinking block"` 这类拒绝往往本来就没有块可剥，于是「关掉顶层思考」这一步永远跑不到，整流做了一半。(b) 思考有两个开关：`thinking.type` 管开不开、`output_config.effort` 管用多大力，新版 Claude Code 两个一起发（8139/8434 条是 `thinking:{"type":"adaptive"}` + `output_config:{"effort":"high"|"xhigh"}`，老写法 `enabled` + `budget_tokens` 只剩 4 条），只关一个等于没关。

**做法** 先剥块，再**无条件**判孤儿；要关就两个一起关，`output_config` 被清空才整个删掉（`format` 等别的字段留着）。客户端已经写了 `thinking:{"type":"disabled"}` 时不删那个字段——删掉等于把「不要思考」这句话也删了，默认开思考的上游会重新打开。

**这是直通路径的整流；转换路径不走这里** 转换路径由 `build_request` 直接决定写什么（A-5），关的时候显式写 `{"type": "disabled"}`。两条路的共同前提是一样的：**省掉 `thinking` 字段不等于关掉思考**，现在的模型默认是开的。

**参照** cc-switch 的 `thinking_rectifier.rs::rectify_anthropic_request` 也是无条件判孤儿，但它的 `should_remove_top_level_thinking` 只认 `type == "enabled"`，照搬到我们这儿在 100% 的流量上都是空转。

### C-12 结构化输出两个方向都走原生字段

**位置** `anthropic.rs::parse_output_format`（读）、`anthropic.rs::build_request` 里写 `output_config`、`anthropic.rs::schema_instruction`（系统提示兜底）。

**坑** `output_config.format` 与 Chat 的 `response_format`、Responses 的 `text.format` 说的是同一件事，形状也已经是 IR 的扁平形状。原来 Anthropic 侧读只读了 `output_config.effort`，`format` 整个漏掉（Claude Code 的结构化输出转到 OpenAI 侧直接消失，2026-09-03 抽样 4 条带 `format`）；写又只拼了一段系统提示，没写原生字段。

**做法** 读进来直接搬，认不出的类型丢掉不猜。写出去写原生 `output_config.format`，**不再**同时拼系统提示——同一个要求说两遍是噪音。两条路互斥：写得出原生字段就只写字段，写不出才退回提示。

**`output_config` 是两件事共用的** `format`（结构化输出）和 `effort`（思考强度，A-5）都挂在这个对象下面，`build_request` 里先把两者攒进同一个 map 再一次性写出去。改任何一边都别把整个 `output_config` 覆盖掉。

**只写实测见过的形状** `{"type": "json_schema", "schema": …}`（`native_output_format`）。`name` / `strict` 是 OpenAI 侧的字段，Anthropic 的真实请求里没出现过，多塞一个猜错就是整个请求 400。`json_object` 同理没见过原生形状，那种情况仍然只发系统提示——`schema_instruction` 因此不能删。要放宽这个限制，先在真实请求里确认，别按模型名猜（`strip_unsupported` 只剥字符串形式的 `format`，见 C-7，这里的对象形式不受影响）。

**注意** `system` 出去之前会被 `inject_cache_control` 变成内容块数组，不是纯字符串——读它的时候别按字符串取。

### C-13 OpenAI 侧的用量有「缓存写入」这一格，且它已经算在输入里

**位置** `openai_responses.rs::parse_usage` / `build_usage`、`openai_chat.rs` 同两个函数、`proxy.rs::apply_openai_usage`。

**坑** OpenAI 官方文档的用量里没有「缓存写入」，所以三处都只读了 `cached_tokens`，缓存写入被当成普通输入。Anthropic 的缓存写入贵 25%，等于少算钱。而且这不只是转换层的事——`apply_openai_usage` 是直通路径也在用的那一个，实测数据库里 7224 条 `openai_responses` 请求的缓存写入**全是 0**。

**实测规模** 真实 Responses 兼容端点普遍会给：8919 次 `response.completed` 里 8040 次带 `input_tokens_details.cache_write_tokens`，另有 124 次用别名 `cached_creation_tokens`。这批语料里被漏记的量是 1198629 个 token（139 次请求带非零值）。

**做法** 两个名字都认；**从 `input_tokens` 里减掉**，和 `cached_tokens` 一样。这一点是实测确认的，不是推的：139 个带非零缓存写入的响应里，`cached + cache_write <= input_tokens` 139/139 成立，减完的余数稳定在 400–500（就是没命中缓存的那点新增内容）。

**别改回去** 别改成「加上去」——会把输入重复计一遍。写回给客户端那一格也别删：`cache_write_tokens` 虽然不是官方字段，但真实上游本来就这么发给 Codex（直通路径上客户端一直在收），不写网关自己的统计就丢掉这部分。

**同类** `anthropic.rs::parse_usage` 原来把 `reasoning` 写死成 0，漏掉了 `output_tokens_details.thinking_tokens`（实测 449 次）。它和 OpenAI 的 `reasoning_tokens` 是同一件事：都已经算在 `output_tokens` 里，只是明细，不用减。

### C-14 `Shape` 要能同时记住「custom」和「namespace」

**位置** `openai_responses.rs::Shape` / `push_tool` / `tool_call_item`。

**坑** Codex 把工具按 namespace 分组（`functions/exec`、`collaboration/followup_task`），转出去时 namespace 一律丢掉、只留裸名，回程靠 `ToolShapes` 补回来。原来的 `Shape` 是四选一的枚举，`Shape::Custom(String)` 里没有 namespace 的位置，而 `functions/exec` 同时是 custom 工具**和** namespace 下的子工具——判断顺序上 custom 优先，namespace 就被扔了，还原出来的 `custom_tool_call` 少一个 `namespace` 字段，Codex 对不上。

**实测规模** 声明 `functions/exec`（custom + 在 namespace 下）的请求 2952 条；历史里带 `namespace` 的 `custom_tool_call` 1220 条；上游响应里 18 条。占全部 `custom_tool_call`（324651 条）的 0.4%，但这是 Codex 较新的用法。

**做法** `Shape::Custom { name, namespace }`，namespace 和形状并存，`tool_call_item` 有 namespace 就补上这一格。

**别改回去** 别把 namespace 挤回同一个枚举位。`ToolSearch` / `LocalShell` 目前没见过挂在 namespace 下的，所以没给它们加——真见到了，做法照 `Custom` 抄。

## 四、查过是误报的，别再追

三条来自代码审查的怀疑，逐条核过之后确认不成立。写在这里是为了别再花时间。

### 官方 Responses 不强制要求 `strict`

怀疑：我们在 Anthropic → Responses 时没写 `strict` 字段，官方端点会拒。

结论：不会。`strict` 省略时按非严格模式处理，不是必填。

### `developer` 上提不会打断 Anthropic 的前缀缓存

怀疑：`openai_responses.rs::parse_request` 把会话中间的 `developer` 项合并进顶层 `system`（B-3 同类），会让 Anthropic 的 prefix cache 每轮都失效。

结论：不会。合并进的是顶层 `system`，位于前缀的最前面且内容稳定，缓存断点不在这里。

### `agent_message` → `Role::User` 是合理降级

怀疑：这个映射是错的。

结论：是有意的降级，目标协议里没有对应角色，降成 user 是三个选项里最好的（同 B-3 的推理）。

### 原 B-2「`temperature` / `top_p` 无条件写入」已不成立

怀疑：`openai_chat.rs::build_request` 不管源请求有没有给都写这两个字段，带 `reasoning` 的端点会 400。

结论：现在的代码是 `if let Some(value)` 守卫的，只有源请求真给了才写。那条描述对应的是更早的版本。

### Gemini 审查里的「100% 确定会 400」是从规范推的，不是实测

那条说的是孤儿 `tool_use` 必然触发 Anthropic 400。规范上确实如此，但语料里没有观测到实际的 400 响应（语料是直通抓的，直通路径不会产生孤儿）。不管怎样 C-1 的展平已经绕开了它。

## 五、语料能证明什么、不能证明什么

语料在 `~/.ccg-gateway/request-bodies/<日期>/`，每个请求四到六份文件：

| 文件 | 内容 | 数量 |
| --- | --- | --- |
| `*-client.body` | 客户端发来的请求体 | 21977 |
| `*-forward.body` | 我们转发给上游的请求体 | 21977 |
| `*-provider.body` | **上游真正回给我们的响应**（流式就是整条 SSE） | 21795 |
| `*-*.headers` | 对应的请求头/响应头 | — |

外加 `~/.ccg-gateway/ccg_logs.db` 的 `request_logs` 表（32030 行），有状态码、错误信息、用量、耗时，按文件名里的数字 join。

**能证明的：**

- 客户端请求长什么样——字段出现频率、取值分布、消息结构，第一节和第三节标「实测规模」的数字都出自这里
- **上游的流长什么样**——事件名、事件顺序、终止标记有没有、用量字段、私有 item 类型。B-6 / B-9 / C-3 / C-4 / C-9 / C-10 / C-13 的数字都是从 `*-provider.body` 量出来的，不是从代码推的
- 请求方向的转换结果对不对——把 `parse_request` → `build_request` 的消息拼装照代码重写一遍跑真实语料，能验角色交替、工具调用与结果的配对、工具名是否已声明。实测 8432 条 Claude Code 请求转 Chat、9397 条 Codex 请求转 Anthropic，结构违规 0 例

**不能证明的：**

- **转换路径本身一条都没跑过。** 把三天里每个请求的 `*-client.body` 和 `*-forward.body` 按协议分类比对，21807 对全部同协议（responses→responses 9715、anthropic→anthropic 8258、chat→chat 265，其余分类不出来的 3569 对两边也一致）。`request_logs` 里也一样：`/responses → /v1/responses`、`/v1/messages → /v1/messages`，没有一条跨协议。所以「我们转出去的请求上游收不收」「我们转出去的响应客户端认不认」这两件事**没有任何实测支撑**
- 因此第二、三节里凡是讲「上游会拒/客户端会认」的结论，来源只有两个：上游文档，和上面那种「照代码重写一遍」的模拟

要补上最后这块，只能开着转换跑一段时间，再回来看这批文件。

**安全提示**：`*-forward.headers` 里有上游 API key（`authorization: Bearer sk-...`），`*-client.headers` 里有客户端凭证。读语料做统计时不要把这些值打到输出里，也不要把 headers 文件当测试用例提交进仓库。

**安全提示**：`*-forward.headers` 文件里有上游 API key（`authorization: Bearer sk-...`）。读语料做统计时不要把这些值打到输出里。

## 六、本文替代的文件

这份文档合并了以下四份的内容，它们描述的是 `9b26a60` 时的状态，其中六条此后已修（第三节）：

- `PROTOCOL_TRANSLATION_TODO.md`（git 已跟踪）
- `cc-review.md`、`grok-review.md`、`Gemini-review.md`（均未跟踪）

删不删由你决定，本文没有依赖它们。




