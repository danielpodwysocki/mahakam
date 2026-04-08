FROM rust:alpine AS build

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY ./ /app/source
WORKDIR /app/source

RUN cargo build --release --bin api

# Squash all layers for a smaller image
FROM scratch AS prod

COPY --from=build /app/source/target/release/api /api

EXPOSE 3000
CMD ["/api"]
