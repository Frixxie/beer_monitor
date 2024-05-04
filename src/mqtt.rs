use anyhow::Result;
use chrono::NaiveDateTime;
use log::{debug, info, warn};
use reqwest::blocking::Client;
use rumqttc::{Connection, Event, Packet};
use serde::{Deserialize, Serialize};

use crate::hem::{DeviceId, SensorIds};

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

impl Measurement {
    pub fn new(device: i32, sensor: i32, measurement: f32) -> Self {
        Self {
            device,
            sensor,
            measurement,
        }
    }
}

pub fn store_measurement(
    client: &reqwest::blocking::Client,
    url: &str,
    entry: SensorEntry,
    device_id: &DeviceId,
    sensor_ids: &SensorIds,
) -> Result<()> {
    let ds18b20 = Measurement::new(*device_id, sensor_ids.ds18b20, entry.ds18b20.temperature);
    let dht11_temperature = Measurement::new(
        *device_id,
        sensor_ids.dht11_temperature,
        entry.dht11.temperature,
    );
    let dht11_humidity =
        Measurement::new(*device_id, sensor_ids.dht11_humidity, entry.dht11.humidity);
    let dht11_dew_point = Measurement::new(
        *device_id,
        sensor_ids.dht11_dew_point,
        entry.dht11.dew_point,
    );

    client.post(url).json(&ds18b20).send()?;
    client.post(url).json(&dht11_temperature).send()?;
    client.post(url).json(&dht11_humidity).send()?;
    client.post(url).json(&dht11_dew_point).send()?;
    Ok(())
}

pub fn handle_connection(
    mut connection: Connection,
    http_client: &Client,
    device_id: &DeviceId,
    sensor_ids: &SensorIds,
    url: &str,
) -> Result<()> {
    for item in connection.iter() {
        match item {
            Ok(item) => match item {
                Event::Incoming(inc) => {
                    if let Packet::Publish(p) = inc {
                        let payload = String::from_utf8(p.payload.to_vec())?;
                        info!("Got payload! {}", payload);
                        match serde_json::from_str::<SensorEntry>(&payload) {
                            Ok(sensor) => {
                                store_measurement(
                                    http_client,
                                    &format!("{}/api/measurements", url),
                                    sensor,
                                    device_id,
                                    sensor_ids,
                                )
                                .unwrap();
                            }
                            Err(e) => {
                                warn!("Error = {:?}", e);
                            }
                        }
                    }
                }
                Event::Outgoing(out) => {
                    debug!("Sending {:?}", out)
                }
            },
            Err(e) => {
                warn!("Error = {:?}", e);
                continue;
            }
        }
    }
    Ok(())
}
