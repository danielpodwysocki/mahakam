# syntax=docker/dockerfile:1
FROM rust:alpine AS build

RUN apk add --no-cache musl-dev

WORKDIR /app

# ── Layer 1: dependency caching ───────────────────────────────────────────────
# Copy only manifest files and build stub sources so that this layer is only
# invalidated when Cargo.toml / Cargo.lock change, not on every .rs edit.
COPY Cargo.toml Cargo.lock ./
COPY crates/api/Cargo.toml    crates/api/Cargo.toml
COPY crates/shared/Cargo.toml crates/shared/Cargo.toml
RUN mkdir -p crates/api/src crates/shared/src \
 && echo ''            > crates/shared/src/lib.rs \
 && echo 'fn main(){}' > crates/api/src/main.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin api; true

# ── Layer 2: application build ────────────────────────────────────────────────
# Copy real source. Only crates/ changes re-run this step; external deps are
# already compiled in the cache mount above.
COPY crates/ ./crates/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    find crates -name "*.rs" | xargs touch \
 && cargo build --release --bin api \
 && cp target/release/api /api

# ── Runtime image ─────────────────────────────────────────────────────────────
FROM alpine:3.21 AS prod

RUN apk add --no-cache curl bash ca-certificates openssl

# kubectl
ARG KUBECTL_VERSION=v1.31.0
RUN curl -LO "https://dl.k8s.io/release/${KUBECTL_VERSION}/bin/linux/amd64/kubectl" \
    && chmod +x kubectl && mv kubectl /usr/local/bin/

# helm
RUN curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash

COPY --from=build /api /api
COPY workspaces/ /app/workspaces/

EXPOSE 3000
ENV WORKSPACES_BASE_PATH=/app/workspaces/base
CMD ["/api"]
