ARG BASE_IMAGE=dockcross/linux-arm64
FROM ${BASE_IMAGE}

ARG IMAGE_NAME
ARG RUST_TARGET
ARG GNU_HOST

ENV DEFAULT_DOCKCROSS_IMAGE=${IMAGE_NAME}
ENV RUST_TARGET=${RUST_TARGET}
ENV GNU_HOST=${GNU_HOST}
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:${PATH}
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_PATH=/opt/libgpiod/${GNU_HOST}/lib/pkgconfig
ENV PKG_CONFIG_SYSROOT_DIR=/
ENV LIBGPIOD_LIB_DIR=/opt/libgpiod/${GNU_HOST}/lib
ENV LIBGPIOD_INCLUDE_DIR=/opt/libgpiod/${GNU_HOST}/include
ENV BINDGEN_EXTRA_CLANG_ARGS=--sysroot=/usr/xcc/${GNU_HOST}/${GNU_HOST}/sysroot

RUN apt-get update \
    && apt-get install -y --no-install-recommends autoconf automake autoconf-archive ca-certificates curl libclang-dev libtool pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable \
    && rustup target add "${RUST_TARGET}"

ARG LIBGPIOD_VERSION=2.2.2
RUN git clone --depth 1 --branch "v${LIBGPIOD_VERSION}" https://git.kernel.org/pub/scm/libs/libgpiod/libgpiod.git /tmp/libgpiod \
    && cd /tmp/libgpiod \
    && ./autogen.sh --host="${GNU_HOST}" --prefix="/opt/libgpiod/${GNU_HOST}" --disable-tools --disable-tests --disable-bindings \
    && make -j"$(nproc)" \
    && make install \
    && rm -rf /tmp/libgpiod

ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=/usr/xcc/arm-unknown-linux-gnueabihf/bin/arm-unknown-linux-gnueabihf-gcc
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=/usr/xcc/aarch64-unknown-linux-gnu/bin/aarch64-unknown-linux-gnu-gcc
