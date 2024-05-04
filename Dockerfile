FROM rust:latest as build-stage
WORKDIR /usr/src/app
COPY . .
RUN cargo install --path .

FROM rust:slim
COPY --from=build-stage /usr/local/cargo/bin/beer_monitor /usr/local/bin/beer_monitor
CMD ["beer_monitor"]
