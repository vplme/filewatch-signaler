# Stage 1: Build the application
FROM rust:1.73-alpine as builder

# Create a new empty shell project
RUN USER=root cargo new --bin app
WORKDIR /app

RUN apk add --no-cache musl-dev

# Copy the Cargo.toml and Cargo.lock files and build the dependencies to cache them
COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo build --release
RUN rm src/*.rs

# Now that the dependencies are built, copy your source code
COPY ./src ./src

# Build the application
RUN touch src/main.rs
RUN cargo build --release

# Stage 2: Prepare the final image
FROM alpine

# Copy the build artifact from the build stage
COPY --from=builder /app/target/release/filewatch-signaler .

# Set the binary as the entrypoint of the container
ENTRYPOINT ["./filewatch-signaler"]