# LLM Mux — 多阶段构建
# 阶段 1: 编译
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release && \
    strip target/release/llm-mux

# 阶段 2: 最小运行时
FROM gcr.io/distroless/cc-debian12
COPY --from=builder /build/target/release/llm-mux /usr/local/bin/llm-mux
COPY config.example.yaml /etc/llm-mux/config.yaml
USER 65534:65534
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/llm-mux", "start", "--config", "/etc/llm-mux/config.yaml"]
