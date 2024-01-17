use color_eyre::eyre::Result;
use tracing::info;
use tracing_subscriber::prelude::*;

use promriak::{
    APP_NAME,
    APP_VERSION,
    APP_DESCRIPTION,
    config::{self, Config}
};

fn main() -> Result<()> {
    color_eyre::install()?;

    let cmd = clap_setup();
    let matches = cmd.get_matches();
    let config_file = matches.get_one::<String>("config");

    let (config, config_file) = config::load_config(config_file)?;

    tracing_setup(&config);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    info!(version = APP_VERSION, bind_address = ?config.bind_address, 
          listener_port = config.listener_port, configuration = config_file,
          "{} starting", APP_NAME);

    rt.block_on(promriak::rt_main(config))?;

    Ok(())
}

fn tracing_setup(config: &Config) {
    let filter = tracing_subscriber::filter::Targets::new()
        .with_target(APP_NAME, config.tracing_level);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json();

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}

fn clap_setup() -> clap::Command {
    clap::Command::new(APP_NAME)
        .bin_name(APP_NAME)
        .version(APP_VERSION)
        .about(APP_DESCRIPTION)
        .next_line_help(true)
        .arg(
            clap::Arg::new("config")
                .long("config")
                .short('c')
                .env("PROMRIAK_CONFIG")
                .help("Optional configuration file")
                .value_name("path to configuration file")
                .value_hint(clap::ValueHint::FilePath)
        )
}
