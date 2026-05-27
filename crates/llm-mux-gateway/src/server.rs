use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;

use llm_mux_core::codec::ConfigAuthenticator;
use llm_mux_core::codec::ConfigurableRouter;

use crate::config::Config;
use crate::handlers;
use crate::middleware;

pub struct Server {
    config: Config,
    router: Arc<ConfigurableRouter>,
    authenticator: Arc<ConfigAuthenticator>,
}

impl Server {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let router = Arc::new(config.to_router().map_err(|e| format!("router: {e}"))?);
        let authenticator = Arc::new(ConfigAuthenticator::new(config.api_keys.clone()));
        Ok(Self {
            config,
            router,
            authenticator,
        })
    }

    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        let bind_addr = format!("{}:{}", self.config.host, self.config.port);
        info!("gateway listening on {}", bind_addr);

        let drain_timeout = self.config.drain_timeout_secs;

        let app_state = handlers::AppState {
            router: self.router,
            authenticator: self.authenticator.clone(),
        };

        let app = Router::new()
            .route("/health", axum::routing::get(handlers::health))
            .route(
                "/v1/chat/completions",
                axum::routing::post(handlers::chat_completions),
            )
            .route("/v1/messages", axum::routing::post(handlers::messages))
            .route("/v1/responses", axum::routing::post(handlers::responses))
            .layer(axum::middleware::from_fn(|req, next| async move {
                middleware::request_id_middleware(req, next).await
            }))
            .with_state(app_state);

        let listener = TcpListener::bind(&bind_addr).await?;

        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                #[cfg(unix)]
                {
                    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                        .expect("failed to install SIGTERM handler");
                    let sigterm = sigterm.recv();
                    tokio::pin!(sigterm);
                    sigterm.await;
                }
                #[cfg(not(unix))]
                {
                    signal::ctrl_c().await.ok();
                }
                info!(
                    "shutdown signal received, draining for {}s...",
                    drain_timeout
                );
                tokio::time::sleep(std::time::Duration::from_secs(drain_timeout)).await;
            })
            .await?;

        Ok(())
    }
}
