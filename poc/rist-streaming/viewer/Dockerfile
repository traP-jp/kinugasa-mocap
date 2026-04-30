FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates git npm \
  && rm -rf /var/lib/apt/lists/*

RUN git clone https://github.com/webrtc/samples.git /app
  
WORKDIR /app

RUN npm install
