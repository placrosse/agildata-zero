FROM ubuntu:latest

RUN \
  apt-get update && \
  apt-get -y upgrade && \
  apt-get -y install vim wget

# Install AgilData Zero
RUN wget https://github.com/AgilData/agildata-zero/releases/download/v0.1.0-test3/agildata-zero-v0.1.0-test3-x86_64-unknown-linux-musl.tar.gz
RUN tar xzf agildata-zero-v0.1.0-test3-x86_64-unknown-linux-musl.tar.gz
COPY start.sh /agildata-zero/
COPY log.toml /agildata-zero/

EXPOSE 3307
CMD /agildata-zero/start.sh 
