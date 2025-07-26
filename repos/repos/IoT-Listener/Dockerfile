# syntax=docker/dockerfile-upstream:master
FROM --platform=$BUILDPLATFORM rust:1.86-slim AS build

WORKDIR /usr/src

ARG TARGETPLATFORM
ARG BUILDPLATFORM
ARG DEBIAN_FRONTEND=noninteractive

# Using musl to ensure the binary is statically linked
# String replacement for target platform: linux/arm64 -> aarch64-unknown-linux-musl
ENV TARGET=${TARGETPLATFORM//linux\/arm64/aarch64-unknown-linux-musl}
# Further string replacement for target platform: linux/amd64 -> x86_64-unknown-linux-musl
ENV TARGET=${TARGET//linux\/amd64/x86_64-unknown-linux-musl}

# Dependencies for cross-compilation
RUN apt-get update && apt-get install -y build-essential cmake musl-tools curl gcc-aarch64-linux-gnu libssl-dev

# Add the target to the rustup toolchain
RUN rustup target add $TARGET

RUN USER=root cargo new rust_listener

WORKDIR /usr/src/rust_listener

COPY Cargo.toml Cargo.lock ./

# Defined environment variables for rust compilation: x86_64-unknown-linux-musl -> rust-lld, aarch64-unknown-linux-musl -> aarch64-linux-gnu-gcc
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc

COPY src ./src

RUN cargo install --target $TARGET --path .

FROM scratch

COPY --from=build /usr/local/cargo/bin/rust_listener .
# Copy SSL certificates for local issuer verification
COPY --from=build /etc/ssl /etc/ssl

CMD ["./rust_listener"]