# CLI 契约

**Feature**: LLM Mux 三协议互转网关 | **Date**: 2026-05-27

## 命令结构

```text
llm-mux <COMMAND> [OPTIONS]

命令:
  start    启动 HTTP 网关服务
  stop     停止运行中的守护进程
  config   配置文件管理
  help     打印帮助信息
```

---

## start — 启动服务

```text
llm-mux start [OPTIONS]

选项:
  -p, --port <PORT>          HTTP 监听端口 [env: LLM_MUX_PORT] [default: 8080]
  -H, --host <HOST>          绑定地址 [env: LLM_MUX_HOST] [default: 127.0.0.1]
  -c, --config <PATH>        配置文件路径 [env: LLM_MUX_CONFIG] [default: config.yaml]
      --log-level <LEVEL>    日志级别: error|warn|info|debug|trace [env: LLM_MUX_LOG_LEVEL] [default: info]
      --drain-timeout <SECS> 优雅关闭超时秒数 [env: LLM_MUX_DRAIN_TIMEOUT] [default: 30]
  -d, --daemon               以守护进程模式运行（后台运行，写入 PID 文件）
      --pid-file <PATH>      PID 文件路径 [default: /var/run/llm-mux.pid]
  -h, --help                 打印帮助信息
```

### 守护进程模式 (`--daemon`)

- **Linux/macOS**: fork 子进程 → 父进程退出 → 子进程写入 PID 文件并继续运行
- **停止方式**: 使用 `llm-mux stop` 或 `kill -TERM $(cat /var/run/llm-mux.pid)`
- **优雅关闭**: 收到 SIGTERM 后等待 `drain_timeout` 秒完成进行中请求再退出
- **PID 文件**: 启动时检查 PID 文件是否已存在，若存在且进程存活则拒绝启动
- **Windows**: `--daemon` 参数被忽略（无 fork 语义），始终前台运行

### 示例

```bash
# 前台运行
llm-mux start --port 9090 --log-level debug

# 守护进程运行
llm-mux start --daemon --config /etc/llm-mux/config.yaml

# 使用环境变量
export LLM_MUX_PORT=8080
llm-mux start
```

---

## stop — 停止服务

```text
llm-mux stop [OPTIONS]

选项:
      --pid-file <PATH>      PID 文件路径 [default: /var/run/llm-mux.pid]
      --timeout <SECS>       等待超时秒数 [default: 10]
  -h, --help                 打印帮助信息
```

### 行为

1. 读取 PID 文件获取进程 ID
2. 向进程发送 SIGTERM 信号
3. 等待进程退出（最多 `timeout` 秒）
4. 超时后发送 SIGKILL 强制终止
5. 删除 PID 文件

### 示例

```bash
llm-mux stop
llm-mux stop --pid-file /tmp/llm-mux.pid --timeout 30
```

---

## config — 配置管理

```text
llm-mux config <SUBCOMMAND>

子命令:
  init      生成默认配置文件
  validate  校验配置文件语法和完整性
  show      打印当前解析后的配置
```

### config init — 生成配置文件

```text
llm-mux config init [OPTIONS]

选项:
  -p, --path <PATH>   输出路径 [default: config.yaml]
  -f, --force         覆盖已存在的文件
  -h, --help          打印帮助信息
```

生成包含所有字段注释的默认配置文件模板。

### config validate — 校验配置

```text
llm-mux config validate [OPTIONS]

选项:
  -p, --path <PATH>   配置文件路径 [default: config.yaml]
  -h, --help          打印帮助信息
```

校验配置文件：
- YAML 语法正确性
- 必填字段完整性（`providers` 非空，每个 provider 含 `protocol`/`base_url`/`api_key`；`routes` 非空）
- 路由中引用的 `provider` 名称必须在 `providers` 中存在
- `model_mapping` 中 key 不得为空字符串
- 协议值是否合法（`openai-chat` / `openai-responses` / `anthropic`）
- 最后一条路由必须为 `models: ["*"]` 作为兜底，且不得包含 `stream`/`has_tools`/`has_media` 条件
- API Key 环境变量引用是否可解析

### config show — 显示配置

```text
llm-mux config show [OPTIONS]

选项:
  -p, --path <PATH>   配置文件路径 [default: config.yaml]
  -h, --help          打印帮助信息
```

打印解析后完整的配置（敏感字段如 `api_key` 脱敏显示为 `***`）。

### 示例

```bash
# 生成默认配置
llm-mux config init

# 校验配置
llm-mux config validate --path prod.yaml

# 查看当前配置
llm-mux config show
```

---

## 配置文件 (config.yaml)

```yaml
# 服务配置
host: "127.0.0.1"
port: 8080
log_level: info
drain_timeout_secs: 30

# API Key 白名单（可选，不配置则不验证入站请求）
api_keys:
  - "sk-xxxxxxxx"

# 上游 Provider 定义（集中配置后端凭证、连接信息和模型映射）
providers:
  openai:
    protocol: openai-chat           # openai-chat | openai-responses | anthropic
    base_url: "https://api.openai.com/v1"
    api_key: "${OPENAI_API_KEY}"
    model_mapping:                  # 入站模型名 → 后端实际模型名（可选）
      "*": "gpt-4o"                #   * 通配符表示兜底映射

  anthropic:
    protocol: anthropic
    base_url: "https://api.anthropic.com"
    api_key: "${ANTHROPIC_API_KEY}"
    model_mapping:
      "gpt-4o": "claude-sonnet-4-6"
      "gpt-4o-mini": "claude-haiku-4-6"
      "o3": "claude-opus-4-6"
      "*": "claude-sonnet-4-6"

  azure-gpt4:
    protocol: openai-chat
    base_url: "https://your-resource.openai.azure.com"
    api_key: "${AZURE_API_KEY}"
    headers:
      api-key: "${AZURE_API_KEY}"

# 路由规则（从上到下匹配，首个命中生效；兜底规则写在最后）
routes:
  # 高优先级: 流式 gpt-4o Chat 请求走 Azure
  - models: ["gpt-4o"]
    protocol: openai-chat
    stream: true
    provider: azure-gpt4

  # 中优先级: Claude 模型走 Anthropic
  - models: ["claude-*"]
    provider: anthropic

  # 低优先级: 其余走 OpenAI 兜底
  - models: ["*"]
    provider: openai
```

### 路由匹配规则

路由按配置文件**从上到下按首个命中生效**：
- 自上而下依次检查；**第一个匹配的规则生效**，后续不再检查
- 规则中的每个匹配条件 AND 关系——全部满足才命中
- 兜底规则 `models: ["*"]` 必须放在**最后**
- 若无可匹配规则，返回 502 错误

| 匹配条件 | 类型 | 必填 | 描述 |
|----------|------|------|------|
| `models` | `string[]` | 是 | 匹配模型名，支持 `*`（任意字符）、`?`（单字符）通配符 |
| `protocol` | `string` | 否 | 匹配入站协议: `openai-chat` / `openai-responses` / `anthropic` |
| `stream` | `bool` | 否 | 匹配流式/非流式请求 |
| `has_tools` | `bool` | 否 | 匹配是否包含工具定义 |
| `has_media` | `bool` | 否 | 匹配是否包含图片/文档等媒体内容 |
| `provider` | `string` | 是 | 命中后路由到的 Provider 名称（引用 `providers` 键） |

### Provider 字段

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `protocol` | `string` | 是 | 目标协议: `openai-chat` / `openai-responses` / `anthropic` |
| `base_url` | `string` | 是 | 目标 API 地址 |
| `api_key` | `string` | 是 | API Key，支持 `${ENV_NAME}` 环境变量展开 |
| `headers` | `map<string,string>` | 否 | 额外 HTTP 请求头 |
| `model_mapping` | `map<string,string>` | 否 | 入站模型名 → 后端实际模型名映射，支持 `*` 通配符，按 key 的 specificity 匹配 |

### 模型映射 (`model_mapping`)

Provider 级别的模型名转换，在路由命中后、编码请求前执行：

```yaml
providers:
  anthropic:
    model_mapping:
      "gpt-4o": "claude-sonnet-4-6"   # 精确匹配优先
      "gpt-4o-mini": "claude-haiku-4-6"
      "*": "claude-sonnet-4-6"         # 通配符兜底
```

- **匹配顺序**: 精确匹配 → 最长通配符前缀匹配 → `*` 兜底
- **未配置**: 未配置 `model_mapping` 时模型名原样透传
- **作用时机**: 路由决策完成后，编码请求前；不影响路由匹配本身

### 配置优先级

命令行参数 > 环境变量 > 配置文件。示例：

```bash
# port 来自命令行，其余来自 prod.yaml
llm-mux start --port 9090 --config prod.yaml
```

### 环境变量

所有 `api_key` 字段和 `--config` / `--port` 等均支持对应环境变量（见各命令 `[env: ...]` 标注）。
