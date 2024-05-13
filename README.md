# Beer monitor

This is a small background job that listen for sensor events on a mqtt queue provided by a ESP32 running Tasmota with a DS18B20 and a DHT11 attached and posts the data to my [home environment monitor](https://github.com/Frixxie/hemrs) solution

The stack is implemented using Rust and Docker and runs within my private kubernetes cluster

## Requirements

* Mqtt broker
* Instance of hemrs
* Postgres

## How to build and run 

To build
```sh
cargo build
```

To run (with my defaults)
```sh
cargo run
```

For configuration options run
```sh
cargo run -- -h
```
