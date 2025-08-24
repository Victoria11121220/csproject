# syntax=docker/dockerfile-upstream:master
FROM --platform=$BUILDPLATFORM rust:1.88-slim AS base

WORKDIR /usr/src

ARG TARGETPLATFORM
ARG BUILDPLATFORM
ARG DEBIAN_FRONTEND=noninteractive

# Using musl to ensure the binary is statically linked
ENV TARGET=${TARGETPLATFORM//linux\/arm64/aarch64-unknown-linux-musl}
ENV TARGET=${TARGET//linux\/amd64/x86_64-unknown-linux-musl}

# Dependencies for cross-compilation
RUN apt-get update && apt-get install -y \
    build-essential cmake musl-tools curl \
    gcc-aarch64-linux-gnu libssl-dev pkg-config

# Add the target to the rustup toolchain
RUN rustup target add $TARGET

RUN USER=root cargo new app
WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

# Defined environment variables for rust compilation
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc

COPY src ./src

# --- Build collector binary ---
FROM base AS collector-build
RUN cargo install --target $TARGET --bin iot-collector --path .

# --- Build processor binary ---
FROM base AS processor-build
RUN cargo install --target $TARGET --bin iot-processor --path .

# --- Collector runtime image ---
FROM scratch AS collector
COPY --from=collector-build /usr/local/cargo/bin/iot-collector /iot-collector
COPY --from=collector-build /etc/ssl /etc/ssl
CMD ["/iot-collector"]

# --- Processor runtime image ---
FROM scratch AS processor
COPY --from=processor-build /usr/local/cargo/bin/iot-processor /iot-processor
COPY --from=processor-build /etc/ssl /etc/ssl
CMD ["/iot-processor"]