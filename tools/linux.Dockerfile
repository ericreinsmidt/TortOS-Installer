# The Linux build: a .deb and an .rpm for x86_64, made on Ubuntu 22.04 so they
# run on it and anything newer (Debian 12, current Ubuntu and Mint, Fedora).
# Built from the Mac with tools/release.sh; see README.
FROM --platform=linux/amd64 ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
	build-essential curl ca-certificates file pkg-config \
	libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev \
	libssl-dev librsvg2-dev libayatana-appindicator3-dev libxdo-dev \
	&& rm -rf /var/lib/apt/lists/*
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
ENV PATH=/root/.cargo/bin:$PATH
RUN cargo install tauri-cli --version "^2" --locked
WORKDIR /src
