use std::pin::Pin;
use std::task::{Context, Poll};

use axum::response::sse::Event;
use llm_mux_core::codec::Codec;
use llm_mux_core::ir::StreamError;
use llm_mux_core::ir::StreamEventType;
use reqwest::Response as ReqwestResponse;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tracing::{debug, error};

const BOUNDED_CHANNEL_SIZE: usize = 64;

pub fn sse_proxy(
    upstream: ReqwestResponse,
    inbound: Box<dyn Codec>,
    outbound: Box<dyn Codec>,
) -> impl futures::Stream<Item = Result<Event, axum::Error>> {
    let (tx, rx) = mpsc::channel::<Result<Event, axum::Error>>(BOUNDED_CHANNEL_SIZE);

    tokio::spawn(async move {
        if let Err(e) = proxy_sse_events(upstream, &*inbound, &*outbound, tx).await {
            error!("SSE proxy error: {}", e);
        }
    });

    SseStream { rx }
}

async fn proxy_sse_events(
    upstream: ReqwestResponse,
    inbound: &dyn Codec,
    outbound: &dyn Codec,
    tx: mpsc::Sender<Result<Event, axum::Error>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = upstream.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = match chunk_result {
            Ok(c) => c,
            Err(e) => {
                let ir_err = llm_mux_core::ir::IrStreamEvent {
                    event_type: StreamEventType::Error,
                    response: None,
                    index: 0,
                    delta: None,
                    stop_reason: None,
                    usage: None,
                    error: Some(StreamError {
                        error_type: Some("connection_error".into()),
                        code: None,
                        message: Some(e.to_string()),
                        param: None,
                    }),
                };
                forward_event(&tx, inbound, &ir_err).await;
                continue;
            }
        };

        let chunk_str = String::from_utf8_lossy(&chunk);
        buffer.push_str(&chunk_str);

        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            if let Some(data) = line
                .strip_prefix("data:")
                .or_else(|| line.strip_prefix("data: "))
            {
                let data = data.trim();
                if data == "[DONE]" {
                    let stop = llm_mux_core::ir::IrStreamEvent {
                        event_type: StreamEventType::Stop,
                        response: None,
                        index: 0,
                        delta: None,
                        stop_reason: None,
                        usage: None,
                        error: None,
                    };
                    forward_event(&tx, inbound, &stop).await;
                    continue;
                }

                match outbound.decode_stream_event(None, data) {
                    Ok(ir_event) => {
                        forward_event(&tx, inbound, &ir_event).await;
                    }
                    Err(e) => {
                        match inbound.decode_stream_event(None, data) {
                            Ok(ir_event) => {
                                forward_event(&tx, inbound, &ir_event).await;
                            }
                            Err(_) => {
                                debug!("SSE decode error (both codecs): {e}");
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn forward_event(
    tx: &mpsc::Sender<Result<Event, axum::Error>>,
    inbound: &dyn Codec,
    event: &llm_mux_core::ir::IrStreamEvent,
) {
    match inbound.encode_stream_event(event) {
        Ok(s) => {
            let payload = if let Some(data) = s.strip_prefix("data: ") {
                data.trim_end_matches('\n').to_string()
            } else {
                s
            };
            if tx.send(Ok(Event::default().data(payload))).await.is_err() {}
        }
        Err(e) => {
            error!("SSE encode error: {}", e);
        }
    }
}

fn sse_str_to_event(sse_str: &str) -> Result<Event, axum::Error> {
    let trimmed = sse_str.trim();
    if trimmed == "data: [DONE]" {
        return Ok(Event::default().data("[DONE]"));
    }
    if let Some(data) = trimmed
        .strip_prefix("data:")
        .or_else(|| trimmed.strip_prefix("data: "))
    {
        Ok(Event::default().data(data.trim().to_string()))
    } else {
        Err(axum::Error::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid SSE string: {sse_str}"),
        )))
    }
}

struct SseStream {
    rx: mpsc::Receiver<Result<Event, axum::Error>>,
}

impl futures::Stream for SseStream {
    type Item = Result<Event, axum::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}
