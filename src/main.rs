use std::time::Duration;

use anyhow::Result;

use chrono::NaiveDateTime;
use log::{debug, info, warn};
use rumqttc::{Client, Event, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use structopt::StructOpt;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DS18B20 {
    #[serde(rename = "Id")]
    _id: String,
    temperature: f32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DHT11 {
    temperature: f32,
    humidity: f32,
    dew_point: f32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct SensorEntry {
    #[serde(rename = "Time")]
    _time: NaiveDateTime,
    #[serde(rename = "DS18B20")]
    ds18b20: DS18B20,
    #[serde(rename = "DHT11")]
    dht11: DHT11,
    #[serde(rename = "TempUnit")]
    _temp_unit: String,
}

#[derive(Serialize, Debug)]
pub struct Measurement {
    device: i32,
    sensor: i32,
    measurement: f32,
}

fn store_measurement(
    client: &reqwest::blocking::Client,
    url: &str,
    entry: SensorEntry,
    device_id: DeviceId,
    sensor_ids: &SensorIds,
) -> Result<()> {
    let ds18b20 = Measurement::new(device_id, sensor_ids.ds18b20, entry.ds18b20.temperature);
    let dht11_temperature = Measurement::new(
        device_id,
        sensor_ids.dht11_temperature,
        entry.dht11.temperature,
    );
    let dht11_humidity =
        Measurement::new(device_id, sensor_ids.dht11_humidity, entry.dht11.humidity);
    let dht11_dew_point =
        Measurement::new(device_id, sensor_ids.dht11_dew_point, entry.dht11.dew_point);

    client.post(url).json(&ds18b20).send()?;
    client.post(url).json(&dht11_temperature).send()?;
    client.post(url).json(&dht11_humidity).send()?;
    client.post(url).json(&dht11_dew_point).send()?;
    Ok(())
}

#[derive(Debug)]
struct SensorIds {
    ds18b20: i32,
    dht11_temperature: i32,
    dht11_humidity: i32,
    dht11_dew_point: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sensor {
    #[serde(skip_serializing)]
    id: i32,
    name: String,
    unit: String,
}

type DeviceId = i32;

#[derive(Serialize, Deserialize, Debug)]
pub struct Device {
    #[serde(skip_serializing)]
    id: i32,
    name: String,
    location: String,
}

fn fetch_devices(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<Device>> {
    let devices = client.get(url).send()?.json::<Vec<Device>>()?;
    Ok(devices)
}

fn fetch_sensors(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<Sensor>> {
    let devices = client.get(url).send()?.json::<Vec<Sensor>>()?;
    Ok(devices)
}

fn setup_sensor(
    client: &reqwest::blocking::Client,
    url: &str,
    sensor_name: &str,
    sensor_unit: &str,
) -> Result<i32> {
    let sensors = fetch_sensors(client, url)?;
    let device = sensors.iter().find(|d| d.name == sensor_name);
    match device {
        Some(d) => {
            println!("{:?}", d);
            Ok(d.id)
        }
        None => {
            let new_device = Sensor {
                id: 0,
                name: sensor_name.to_string(),
                unit: sensor_unit.to_string(),
            };
            let response = client.post(url).json(&new_device).send()?;
            println!("{:?}", response);
            setup_sensor(&client, url, sensor_name, sensor_unit)
        }
    }
}

fn setup_sensors(client: &reqwest::blocking::Client, url: &str) -> Result<SensorIds> {
    let ds18b20 = setup_sensor(&client, url, "DS18B20", "°C")?;
    let dht11_temperature = setup_sensor(&client, url, "DHT11 Temperature", "°C")?;
    let dht11_humidity = setup_sensor(&client, url, "DHT11 Humidity", "%")?;
    let dht11_dew_point = setup_sensor(&client, url, "DHT11 Dew Point", "°C")?;

    Ok(SensorIds {
        ds18b20,
        dht11_temperature,
        dht11_humidity,
        dht11_dew_point,
    })
}

fn setup_device(
    client: &reqwest::blocking::Client,
    url: &str,
    device_name: &str,
    device_location: &str,
) -> Result<DeviceId> {
    let devices = fetch_devices(client, url)?;
    let device = devices
        .iter()
        .find(|d| d.name == device_name && d.location == device_location);
    match device {
        Some(d) => {
            println!("{:?}", d);
            Ok(d.id)
        }
        None => {
            let new_device = Device {
                id: 0,
                name: device_name.to_string(),
                location: device_location.to_string(),
            };
            let response = client.post(url).json(&new_device).send()?;
            println!("{:?}", response);
            setup_device(&client, url, device_name, device_location)
        }
    }
}

impl Measurement {
    pub fn new(device: i32, sensor: i32, measurement: f32) -> Self {
        Self {
            device,
            sensor,
            measurement,
        }
    }
}

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

    let (client, mut connection) = Client::new(mqttoptions, 10);
    client.subscribe(opts.topic, QoS::AtMostOnce).unwrap();

    // Iterate to poll the eventloop for connection progress
    for item in connection.iter() {
        match item {
            Ok(item) => {
                // println!("Received = {:?}", item);
                match item {
                    Event::Incoming(inc) => match inc {
                        rumqttc::Packet::Publish(p) => {
                            let payload = String::from_utf8(p.payload.to_vec())?;
                            info!("Got payload! {}", payload);
                            match serde_json::from_str::<SensorEntry>(&payload) {
                                Ok(sensor) => {
                                    store_measurement(
                                        &http_client,
                                        &format!("{}/api/measurements", opts.hemrs_base_url),
                                        sensor,
                                        device_id,
                                        &sensor_ids,
                                    )
                                    .unwrap();
                                }
                                Err(e) => {
                                    warn!("Error = {:?}", e);
                                }
                            }
                        }
                        _ => (),
                    },
                    Event::Outgoing(out) => {
                        debug!("Sending {:?}", out)
                    }
                }
            }
            Err(e) => {
                warn!("Error = {:?}", e);
                continue;
            }
        }
    }
    Ok(())
}
