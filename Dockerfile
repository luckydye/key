FROM nailyudha/tauri:latest

WORKDIR /home/nonroot

COPY . .
RUN mise trust ./.mise.toml
RUN mise install
RUN cargo check
RUN bun i --frozen-lockfile

# For building Linux (Support for AMD64 & ARM64)
# RUN bun tauri build

# For building Android (Support only for AMD64)
RUN apt update -y && apt install android-sdk -y
ENV ANDROID_HOME=/usr/lib/android-sdk

RUN bun tauri android init \
  && bun tauri android build --apk

# For building Windows
## AMD64
# RUN bun tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc
## ARM64
# RUN bun tauri build --runner cargo-xwin --target aarch64-pc-windows-msvc
