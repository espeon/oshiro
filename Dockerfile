# Build configuration
ARG project_name=oshiro
# Fill in name of crate^ here

# Set up rust build environment
FROM rust:latest as builder
ARG project_name

# Create layer for the dependencies, so we don't have to rebuild them later
RUN apt-get update \
    && apt-get install -y cmake build-essential
WORKDIR /usr/src
RUN USER=root cargo new $project_name
WORKDIR /usr/src/$project_name
COPY Cargo.toml Cargo.lock ./
COPY .cargo/ ./
RUN cargo build --release
RUN rm src/*.rs

# Build the actual source
COPY src ./src
#COPY graphql ./graphql
#COPY sqlx-data.json ./sqlx-data.json
RUN touch ./src/main.rs && cargo build --release

# Create a "minimal" docker container
FROM debian:buster-slim
ARG project_name
RUN apt-get update \
    && apt-get install -y ca-certificates sudo \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/$project_name/target/release/$project_name ./app
USER 1000
CMD ["./app"]