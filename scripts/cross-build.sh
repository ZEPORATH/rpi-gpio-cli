#!/usr/bin/env bash
set -euo pipefail

target="${1:-}"
case "${target}" in
    pi32)
        base_image="dockcross/linux-armv7"
        image_name="gpio-cli-cross-pi32"
        rust_target="armv7-unknown-linux-gnueabihf"
        gnu_host="arm-unknown-linux-gnueabihf"
        ;;
    pi64)
        base_image="dockcross/linux-arm64"
        image_name="gpio-cli-cross-pi64"
        rust_target="aarch64-unknown-linux-gnu"
        gnu_host="aarch64-unknown-linux-gnu"
        ;;
    *)
        echo "Usage: $0 <pi32|pi64>" >&2
        exit 2
        ;;
esac

docker build \
    --build-arg "BASE_IMAGE=${base_image}" \
    --build-arg "IMAGE_NAME=${image_name}" \
    --build-arg "RUST_TARGET=${rust_target}" \
    --build-arg "GNU_HOST=${gnu_host}" \
    --tag "${image_name}" \
    --file docker/cross/Dockerfile .

wrapper=".dockcross-bin/${target}"
mkdir -p .dockcross-bin cross-target
docker run --rm "${image_name}" > "${wrapper}"
chmod +x "${wrapper}"
bash "${wrapper}" \
    --args "-e CARGO_HOME=/work/cross-target/cargo-home -e CARGO_TARGET_DIR=/work/cross-target" \
    cargo build --release --target "${rust_target}"

echo "Built cross-target/${rust_target}/release/gpio-cli"
