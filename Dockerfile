FROM debian:bullseye-slim

# Create a non-privileged user that the app will run under.
# See https://docs.docker.com/go/dockerfile-user-best-practices/
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

WORKDIR /app

COPY target/aarch64-unknown-linux-gnu/release/bridge-scrims .
COPY Config.toml .

ENTRYPOINT [ "./bridge-scrims" ]
