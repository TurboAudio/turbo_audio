FROM rust:slim
WORKDIR turbo_audio
RUN apt-get update -y
COPY . .
RUN cargo build

