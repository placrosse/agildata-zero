FROM ubuntu:latest

RUN apt-get update
RUN apt-get -y install vim wget

# Install AgilData Zero
RUN mkdir -p /etc/zero.d
RUN wget https://github.com/AgilData/agildata-zero/releases/download/v0.1.0-test3/agildata-zero-v0.1.0-test3-x86_64-unknown-linux-musl.tar.gz
RUN tar xzf agildata-zero-v0.1.0-test3-x86_64-unknown-linux-musl.tar.gz