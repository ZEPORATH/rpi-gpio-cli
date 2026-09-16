FROM rust:1-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends autoconf automake autoconf-archive git libclang-dev libtool pkg-config \
    && rm -rf /var/lib/apt/lists/*

ARG LIBGPIOD_VERSION=2.2.2
RUN git clone --depth 1 --branch "v${LIBGPIOD_VERSION}" https://git.kernel.org/pub/scm/libs/libgpiod/libgpiod.git /tmp/libgpiod \
    && cd /tmp/libgpiod \
    && ./autogen.sh --prefix=/usr/local --disable-tools --disable-tests --disable-bindings \
    && make -j"$(nproc)" \
    && make install \
    && ldconfig \
    && rm -rf /tmp/libgpiod

RUN rustup component add rustfmt

WORKDIR /work
