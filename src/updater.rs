use std::{io::Write, collections::HashMap};
use color_eyre::eyre::Result;
use tracing::{debug, info, warn};
use tokio::time::Instant;
use reqwest::Client;

use crate::{Cache, SharedInstance};

type JsonMap = HashMap<String, serde_json::Value>;

pub fn start(client: Client, instance: SharedInstance) -> tokio::task::JoinHandle<()> {
    // Unwrap is safe, it is guaranteed to contain a value
    let scrape_interval = instance.config.scrape_interval.unwrap().as_millis();
    let stale_threshold = instance.config.stale_threshold.unwrap().as_millis();
    let prefix = instance.config.prefix.as_ref().unwrap();

    info!(instance = instance.config.id, endpoint = instance.config.endpoint,
          scrape_interval, stale_threshold, prefix,
          "updater starting");

    tokio::task::spawn(update_loop(client, instance))
}

async fn update_loop(client: Client, instance: SharedInstance) {
    loop {
        do_update(&client, &instance).await;
        // Unwrap is safe, it is guaranteed to contain a value
        let duration = instance.config.scrape_interval.unwrap();
        tokio::time::sleep(duration).await;
    }
}

async fn do_update(client: &Client, instance: &SharedInstance) {
    let start = Instant::now();
    match get_stats(client, &instance.config.endpoint).await {
        Ok(stats) => {
            process_stats(stats, instance, start).await
        },
        Err(err) => {
            let duration = start.elapsed();
            warn!(instance = ?instance.config.id, ?duration, error = ?err,
                  "get_stats_fail");
        }
    }
}

async fn get_stats(client: &Client, url: &str) -> Result<JsonMap> {
    let stats = client
        .get(url)
        .send()
        .await?
        .json()
        .await?;

    Ok(stats)
}

async fn process_stats(stats: JsonMap, instance: &SharedInstance, start: Instant) {
    match render_stats(stats, instance) {
        Ok(rendered) => {
            let last_update = Instant::now();
            let cache = Some(Cache { last_update, value: rendered });
            {
                let mut lock = instance.cache.write().await;
                *lock = cache;
            }
            let duration = start.elapsed().as_millis();
            debug!(instance = instance.config.id, ?duration,
                  "update_stats_ok")
        },
        Err(err) => {
            warn!(instance = instance.config.id, error = ?err,
                  "update_stats_fail")
        }
    }
}

fn render_stats(stats: JsonMap, instance: &SharedInstance) -> Result<Vec<u8>> {
    // Unwrap is safe, it is guaranteed to contain a value
    let prefix = instance.config.prefix.as_ref().unwrap();

    let mut buf = Vec::new();

    for (metric, value) in stats.iter() {
        if let Some(metrics) = &instance.config.metrics {
            if !metrics.contains(metric) {
                continue
            }
        }

        if !value.is_number() {
            continue
        }

        if let Some(value) = value.as_f64() {
            render_metric(&mut buf, metric, value, prefix)?;
        }
    };

    // Unwrap is safe, it is guaranteed to contain a value
    if instance.config.special_metrics.unwrap() {
        render_special_metrics(&mut buf, stats, prefix)?;
    }

    Ok(buf)
}

fn render_special_metrics(buf: &mut Vec<u8>, stats: JsonMap, prefix: &str) -> Result<()> {
    if let Some(ring_members) = stats.get("ring_members") {
        if let Some(ring_members) = ring_members.as_array() {
            render_metric(buf, "ring_members_count", ring_members.len() as f64, prefix)?
        }
    }

    if let Some(connected_nodes) = stats.get("connected_nodes") {
        if let Some(connected_nodes) = connected_nodes.as_array() {
            render_metric(buf, "connected_nodes_count", connected_nodes.len() as f64, prefix)?;
            render_metric(buf, "available_nodes_count", (connected_nodes.len() + 1) as f64, prefix)?;
        }
    }

    Ok(())
}

#[inline(always)]
fn render_metric(mut buf: &mut Vec<u8>, metric: &str, value: f64, prefix: &str) -> Result<()> {
    writeln!(&mut buf, "# TYPE {prefix}{metric} gauge")?;
    writeln!(&mut buf, "{prefix}{metric} {value}\n")?;
    Ok(())
}
