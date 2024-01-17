use std::{
    time::Duration,
    collections::HashSet,
    net::IpAddr,
    str::FromStr, path::Path
};

use color_eyre::eyre::Result;
use serde::{de::Error, Deserialize, Deserializer};
use serde_with::serde_as;

const CONFIG_FILE_SEARCH_LOCATIONS: [&str; 3] = [
    "promriak.yaml",
    "/usr/local/etc/promriak/promriak.yaml",
    "/etc/promriak/promriak.yaml",
];

const TRACING_LEVEL_DEFAULT: tracing::Level = tracing::Level::INFO;
const BIND_ADDRESS_DEFAULT: &str = "127.0.0.1";
const LISTENER_PORT_DEFAULT: u16 = 9198;
const SCRAPE_INTERVAL_DEFAULT: u64 = 2500;
const STALE_THRESHOLD_DEFAULT: u64 = 20_000;
const PREFIX_DEFAULT: &str = "riak_";
const SPECIAL_METRICS_DEFAULT: bool = true;

const DEFAULT_INSTANCE_ID: &str = "local";
const DEFAULT_INSTANCE_ENDPOINT: &str = "http://127.0.0.1:8098/stats";

#[serde_as]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct InstanceConfig {
    pub id: String,
    pub endpoint: String,
    #[serde_as(as = "Option<serde_with::DurationMilliSeconds<u64>>")]
    pub scrape_interval: Option<Duration>,
    #[serde_as(as = "Option<serde_with::DurationMilliSeconds<u64>>")]
    pub stale_threshold: Option<Duration>,
    pub prefix: Option<String>,
    pub metrics: Option<HashSet<String>>,
    pub special_metrics: Option<bool>,
}

#[serde_as]
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default = "tracing_level_default")]
    #[serde(deserialize_with="to_tracing_level")]
    pub tracing_level: tracing::Level,
    #[serde(default = "bind_address_default")]
    #[serde(deserialize_with="to_ipaddr")]
    pub bind_address: IpAddr,
    #[serde(default = "listener_port_default")]
    pub listener_port: u16,
    #[serde(default = "scrape_interval_default")]
    #[serde_as(as = "serde_with::DurationMilliSeconds<u64>")]
    pub scrape_interval: Duration,
    #[serde(default = "stale_threshold_default")]
    #[serde_as(as = "serde_with::DurationMilliSeconds<u64>")]
    pub stale_threshold: Duration,
    #[serde(default = "prefix_default")]
    pub prefix: String,
    #[serde(default = "instances_default")]
    pub instances: Vec<InstanceConfig>,
    pub metrics: Option<HashSet<String>>,
    #[serde(default = "special_metrics_default")]
    pub special_metrics: bool,
}

fn to_tracing_level<'de, D>(
    deserializer: D
) -> Result<tracing::Level, D::Error>
where
    D: Deserializer<'de>
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    tracing::Level::from_str(s).map_err(D::Error::custom)
}

fn to_ipaddr<'de, D>(
    deserializer: D
) -> Result<IpAddr, D::Error>
where
    D: Deserializer<'de>
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse::<IpAddr>().map_err(D::Error::custom)
}

fn tracing_level_default() -> tracing::Level { 
    TRACING_LEVEL_DEFAULT
}

fn bind_address_default() -> IpAddr { 
    BIND_ADDRESS_DEFAULT.parse::<IpAddr>().unwrap()
}

fn listener_port_default() -> u16 {
    LISTENER_PORT_DEFAULT
}

fn scrape_interval_default() -> Duration {
    Duration::from_millis(SCRAPE_INTERVAL_DEFAULT)
}

fn stale_threshold_default() -> Duration {
    Duration::from_millis(STALE_THRESHOLD_DEFAULT)
}

fn prefix_default() -> String {
    PREFIX_DEFAULT.to_owned()
}

fn instances_default() -> Vec<InstanceConfig> {
    vec![InstanceConfig { 
        id: DEFAULT_INSTANCE_ID.to_owned(), 
        endpoint: DEFAULT_INSTANCE_ENDPOINT.to_owned(), 
        .. Default::default()
    }]
}

fn special_metrics_default() -> bool {
    SPECIAL_METRICS_DEFAULT
}


pub fn load_config(config_file: Option<&String>) -> Result<(Config, Option<String>)> {
    let (file, cfg_data) = get_config(config_file)?;
    let mut config =  serde_yaml::from_slice::<Config>(&cfg_data)?;
    process_instances(&mut config);
    Ok((config, file))
}

fn get_config(config_file: Option<&String>) -> Result<(Option<String>, Vec<u8>)> {
    if let Some(file_path) = config_file {
        let file_path = std::path::Path::new(file_path);
        try_provided_config(file_path)
    } else {
        try_default_config_locations()
    }
}

fn try_provided_config(file_path: &Path) -> Result<(Option<String>, Vec<u8>)> {
    if file_path.is_file() {
        let abs = file_path.canonicalize()?.to_string_lossy().to_string();
        let content = std::fs::read(file_path)?;
        Ok((Some(abs), content))
    } else {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }
}

fn try_default_config_locations() -> Result<(Option<String>, Vec<u8>)> {
    for file_path in CONFIG_FILE_SEARCH_LOCATIONS {
        let path = std::path::Path::new(file_path);
        if path.is_file() {
            let abs = path.canonicalize()?.to_string_lossy().to_string();
            let content = std::fs::read(file_path)?;
            return Ok((Some(abs), content))
        }
    }

    Ok((None, vec![]))

    //Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
}

fn process_instances(config: &mut Config) {
    config.instances.iter_mut().for_each(|ec| {
        if ec.scrape_interval.is_none() {
            ec.scrape_interval = Some(config.scrape_interval)
        }

        if ec.stale_threshold.is_none() {
            ec.stale_threshold = Some(config.stale_threshold)
        }

        if ec.prefix.is_none() {
            ec.prefix = Some(config.prefix.clone())
        }

        if ec.metrics.is_none() {
            ec.metrics = config.metrics.clone()
        }

        if ec.special_metrics.is_none() {
            ec.special_metrics = Some(config.special_metrics)
        }
    });
}
