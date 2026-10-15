# syntax=docker/dockerfile:1
FROM node:20-bookworm AS frontend-builder
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@latest --activate
COPY package.json pnpm-lock.yaml* pnpm-workspace.yaml* ./
RUN pnpm install --frozen-lockfile || pnpm install
COPY . .
RUN pnpm build

FROM rust:1.80-bookworm AS app-builder
WORKDIR /app
RUN apt-get update && apt-get install -y \
    libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libxdo-dev \
    libssl-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    ffmpeg \
    mpv \
    && rm -rf /var/lib/apt/lists/*

COPY --from=frontend-builder /app/dist ./dist
COPY src-tauri ./src-tauri
WORKDIR /app/src-tauri
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y \
    libwebkit2gtk-4.1-0 \
    libayatana-appindicator3-1 \
    ffmpeg \
    mpv \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=app-builder /app/src-tauri/target/release/animated-linux /usr/local/bin/animated-linux
CMD ["/usr/local/bin/animated-linux"]
