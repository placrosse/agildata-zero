FROM ubuntu:latest

RUN \
  apt-get update && \
  apt-get -y upgrade && \
  apt-get -y install vim wget

# Install AgilData Zero
# TODO: Add a link for the latest tarball
# RUN wget https://github.com/AgilData/agildata-zero/releases/download/v0.1.0-test3/agildata-zero-v0.1.0-test3-x86_64-unknown-linux-musl.tar.gz
RUN wget https://github.com/AgilData/agildata-zero/releases/download/v0.1.0-test5/agildata-zero-v0.1.0-test5-x86_64-unknown-linux-musl.tar.gz

RUN tar xzf agildata-zero-v0.1.0-test5-x86_64-unknown-linux-musl.tar.gz
COPY start.sh /agildata-zero/

EXPOSE 3307
CMD /agildata-zero/start.sh 
