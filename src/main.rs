use std::time::Duration;

use anyhow::Result;

use log::info;
use rumqttc::{Client, MqttOptions, QoS};
use simple_logger::SimpleLogger;
use structopt::StructOpt;

use crate::{
    hem::{setup_device, setup_sensors},
    mqtt::handle_connection,
};

mod hem;
mod mqtt;

#[derive(StructOpt)]
pub struct Opts {
    #[structopt(short, long, env, default_value = "server")]
    pub mqtt_host: String,

    #[structopt(short, long, env, default_value = "tele/beer/SENSOR")]
    pub topic: String,

    #[structopt(short, long, env, default_value = "http://localhost:65534")]
    pub hemrs_base_url: String,

    #[structopt(short, long, env, default_value = "Beer")]
    pub device_name: String,

    #[structopt(short = "l", long, env, default_value = "Celar")]
    pub device_location: String,
}

fn main() -> Result<()> {
    let opts = Opts::from_args();
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()?;

    let http_client = reqwest::blocking::Client::new();

    let device_id = setup_device(
        &http_client,
        &format!("{}/api/devices", opts.hemrs_base_url),
        &opts.device_name,
        &opts.device_location,
    )?;

    info!("{:?}", device_id);

    let sensor_ids = setup_sensors(
        &http_client,
        &format!("{}/api/sensors", opts.hemrs_base_url),
    )?;

    info!("{:?}", sensor_ids);

    let mut mqttoptions = MqttOptions::new("beer_collector", opts.mqtt_host, 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, connection) = Client::new(mqttoptions, 10);
    client.subscribe(opts.topic, QoS::AtMostOnce)?;

    handle_connection(
        connection,
        &http_client,
        &device_id,
        &sensor_ids,
        &opts.hemrs_base_url,
    )?;
    Ok(())
}
