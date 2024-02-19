FROM debian:bullseye-slim

WORKDIR /app

COPY target/aarch64-unknown-linux-gnu/release/bridge-scrims .
COPY Config.toml .

ENTRYPOINT [ "./bridge-scrims" ]
