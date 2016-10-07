FROM ubuntu:latest

RUN apt-get update
RUN apt-get -y install git vim build-essential curl wget

# Install AgilData Zero
RUN wget https://github.com/AgilData/agildata-zero/releases/download/v0.1.0-test2/agildata-zero-v0.1.0-test2-x86_64-unknown-linux-musl.tar.gz
RUN tar xzf agildata-zero-v0.1.0-test2-x86_64-unknown-linux-musl.tar.gz