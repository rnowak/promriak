use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use color_eyre::eyre::Result;
use lazy_static::lazy_static;
use tokio::{sync::RwLock, time::Instant};
use tracing::warn;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};

use reqwest::Client;

pub mod config;
use config::{Config, InstanceConfig};

mod updater;

type Instances = HashMap<String, Arc<Instance>>;
type SharedAppState = Arc<AppState>;
type SharedInstance = Arc<Instance>;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

lazy_static! {
    static ref USER_AGENT: String = format!("{}/{}", APP_NAME, APP_VERSION);
}

#[derive(Debug)]
struct Cache {
    last_update: Instant,
    value: Vec<u8>,
}

#[derive(Debug)]
pub struct Instance {
    config: InstanceConfig,
    cache: RwLock<Option<Cache>>,
}

#[derive(Debug)]
struct AppState {
    config: Config,
    instances: Instances,
}

pub async fn rt_main(mut config: Config) -> Result<()> {
    let endpoints = make_endpoints(&mut config);
    start_instance_updaters(&endpoints)?;

    let state = AppState {
        instances: endpoints,
        config,
    };
    start_server(state).await?;

    Ok(())
}

async fn start_server(state: AppState) -> Result<()> {
    let addr = SocketAddr::from((state.config.bind_address, state.config.listener_port));

    let state = Arc::new(state);

    let app = Router::new()
        .route("/health", get(health))
        .route("/stats/:id", get(stats))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

fn make_endpoints(config: &mut Config) -> Instances {
    config
        .instances
        .iter_mut()
        .map(|instance_config| {
            let cache = RwLock::new(Option::None);
            let instance = Instance {
                config: instance_config.clone(),
                cache,
            };
            let instance = Arc::new(instance);

            (instance_config.id.clone(), instance)
        })
        .collect::<Instances>()
}

fn start_instance_updaters(instances: &Instances) -> Result<()> {
    let client = Client::builder()
        .user_agent(USER_AGENT.as_str())
        .timeout(Duration::from_millis(5000))
        .build()?;

    instances.iter().for_each(|(_, instance)| {
        updater::start(client.clone(), instance.clone());
    });

    Ok(())
}

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

async fn stats(Path(id): Path<String>, State(state): State<SharedAppState>) -> impl IntoResponse {
    if let Some(instance) = state.instances.get(&id) {
        // Unwrap is safe, it is guaranteed to contain a value
        let stale_threshold = instance.config.stale_threshold.unwrap();

        let instance_lock = instance.cache.read().await;
        if let Some(cache) = instance_lock.as_ref() {
            let delta = Instant::now().duration_since(cache.last_update);
            if delta < stale_threshold {
                return (StatusCode::OK, cache.value.clone());
            } else {
                warn!(instance = id, age = ?delta, stale_threshold = ?stale_threshold, "stale_read")
            }
        }
    }

    (StatusCode::NOT_FOUND, vec![])
}
