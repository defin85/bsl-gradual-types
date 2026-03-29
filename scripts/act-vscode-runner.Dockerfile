ARG BASE_IMAGE=catthehacker/ubuntu:act-latest
FROM ${BASE_IMAGE}

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libasound2t64 \
        libatk-bridge2.0-0 \
        libatspi2.0-0 \
        libdrm2 \
        libgbm1 \
        libgtk-3-0 \
        libnspr4 \
        libnss3 \
        libx11-xcb1 \
        libxcb-dri3-0 \
        libxcomposite1 \
        libxdamage1 \
        libxfixes3 \
        libxrandr2 \
        libxshmfence1 \
        libxss1 \
    && rm -rf /var/lib/apt/lists/*
