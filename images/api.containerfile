FROM rust:alpine AS build

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY ./ /app/source
WORKDIR /app/source

RUN cargo build --release --bin api

FROM alpine:3.21 AS prod

RUN apk add --no-cache curl bash ca-certificates openssl

# kubectl
ARG KUBECTL_VERSION=v1.31.0
RUN curl -LO "https://dl.k8s.io/release/${KUBECTL_VERSION}/bin/linux/amd64/kubectl" \
    && chmod +x kubectl && mv kubectl /usr/local/bin/

# helm
RUN curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash

COPY --from=build /app/source/target/release/api /api
COPY environments/ /app/environments/

EXPOSE 3000
ENV ENVIRONMENTS_BASE_PATH=/app/environments/base
CMD ["/api"]
