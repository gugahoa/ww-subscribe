# select build image
FROM rustlang/rust:nightly

# create a new empty shell project
RUN USER=root cargo new --lib ww_subscription
WORKDIR /ww_subscription

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/ww_subscription*
RUN cargo build --release

# our final base
FROM rust:latest

# copy the build artifact from the build stage
COPY --from=build /ww_subscription/target/release/ww_subscription .

# set the startup command to run your binary
CMD ["./ww-subscription"]