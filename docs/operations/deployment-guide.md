# 部署指南

## Docker 部署

```bash
# 构建镜像
docker build -t llm-mux .

# 运行
docker run -d \
  --name llm-mux \
  -p 8080:8080 \
  -v $(pwd)/config.yaml:/etc/llm-mux/config.yaml \
  -e OPENAI_API_KEY=sk-xxx \
  -e ANTHROPIC_API_KEY=sk-ant-xxx \
  llm-mux
```

### Dockerfile 结构

```
Stage 1 (builder):
  Base: rust:1.85-slim-bookworm
  Build: cargo build --release
  Strip: target/release/llm-mux

Stage 2 (runtime):
  Base: gcr.io/distroless/cc-debian12
  Binary: /usr/local/bin/llm-mux
  Config: /etc/llm-mux/config.yaml
  User: 65534:65534 (nobody)
  Expose: 8080
```

最终镜像 < 15MB（stripped）。

## 配置

配置文件通过 `--config` 指定，支持 `${ENV_VAR}` 环境变量展开。

```yaml
host: "127.0.0.1"
port: 8080
log_level: info
drain_timeout_secs: 30

api_keys:
  - "sk-xxxxxxxx"          # 可选：API Key 白名单

providers:
  openai:
    format: openai
    url: "https://api.openai.com/v1"
    api_key: "${OPENAI_API_KEY}"

routes:
  - models: ["claude-*"]
    provider: anthropic
  - models: ["*"]
    provider: openai        # 兜底规则必须无条件
```

## 环境变量

| 变量 | 说明 |
|------|------|
| `OPENAI_API_KEY` | OpenAI API Key |
| `ANTHROPIC_API_KEY` | Anthropic API Key |
| `OPENCODE_GO_API_KEY` | OpenCode Go API Key |

## 运维

```bash
# 查看日志
docker logs -f llm-mux

# 优雅停止
llm-mux stop

# 健康检查
curl http://localhost:8080/health

# 测试请求
curl http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-xxx" \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]}'
```
