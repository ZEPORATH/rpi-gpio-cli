# gpio-cli

A small Linux GPIO CLI written in Rust. It uses the safe `libgpiod` crate over
the native libgpiod 2.x C library and the Linux GPIO character-device API.

Pins are GPIO **line offsets**, not Raspberry Pi header pin numbers or BCM names.
The default chip is `/dev/gpiochip0`.

## Commands

```text
gpio-cli [--chip PATH] read <pin>
gpio-cli [--chip PATH] write <pin> <HIGH|LOW|1|0> [--for <duration>]
gpio-cli [--chip PATH] write-all <HIGH|LOW|1|0> [--for <duration>]
gpio-cli [--chip PATH] sequence <config.json>
```

Examples:

```sh
gpio-cli read 17
gpio-cli write 17 HIGH
gpio-cli write 17 HIGH --for 10s
gpio-cli write 17 LOW --for 500ms
gpio-cli write 17 0
gpio-cli write-all LOW --for 10s
gpio-cli --chip /dev/gpiochip4 read 17
```

The `--for` option accepts milliseconds (`500ms`), seconds (`10s`), minutes
(`2m`), or bare seconds (`10`). It keeps the process and libgpiod line request
alive for that duration. Without `--for`, the process releases the line
immediately after writing. Linux does not guarantee an output remains driven
after release.

`write-all` requests every line exposed by the selected GPIO chip and sets each
one as an output. This can affect board functions such as I2C, SPI, UART, power
control, or reset lines. It fails if any requested line is in use by another
consumer, so use it only on a GPIO chip whose lines are safe to control.

### HIL sequence runner

[`hil-sequence.json`](hil-sequence.json) contains the provided relay and reed
configuration. Run it on the Pi with:

```sh
./gpio-cli sequence hil-sequence.json
```

The runner drives every `io_mode: "write"` peripheral with a direct `pin`
HIGH for five seconds, then LOW for five seconds. It prints every transition
and reads each `io_mode: "read"` peripheral at each transition. The sample
runs three cycles; remove `cycles` to repeat until interrupted. The stepper is
not toggled by this relay sequence because it has `pin_dir`, `pin_enable`, and
`pin_step` rather than one direct output pin.

## Native Docker development

No host Rust or libgpiod installation is needed:

```sh
make check
make test
make build
```

The native release binary is written to `target/release/gpio-cli` in the Docker
volume. To run against host GPIO hardware, build first and use a one-shot
container with the GPIO device passed through:

```sh
docker compose run --rm --device /dev/gpiochip0 dev \
  cargo run --release -- read 17
```

The user running Docker still needs permission to access the GPIO device.

### Why libgpiod is built from source

The Rust `libgpiod` crate currently requires libgpiod 2.x. Debian Bookworm's
standard package provides libgpiod 1.6.x, which is too old, so the Docker image
clones the pinned libgpiod 2.2.2 source and builds it natively. This also gives
the Rust binding generator the matching 2.x headers and keeps local builds
consistent with the Raspberry Pi cross-builds.

This is still a native libgpiod integration: the CLI links to the C library and
uses Linux's GPIO character-device API. Building the library in the image only
ensures that the required version, headers, and ABI are available; it does not
replace libgpiod with a Rust GPIO implementation.

## Raspberry Pi cross-compilation with dockcross

Both commands build a custom dockcross image. The image cross-compiles and
installs libgpiod 2.2.2 from source, installs the matching Rust target, and then
links the Rust binary against that target library. Building libgpiod inside the
target image is important: it produces ARM-compatible headers and libraries
instead of accidentally linking the host x86-64 libgpiod.

```sh
make pi32  # Raspberry Pi OS 32-bit, ARMv7 hard-float
make pi64  # Raspberry Pi OS 64-bit, AArch64
```

Outputs:

```text
cross-target/armv7-unknown-linux-gnueabihf/release/gpio-cli
cross-target/aarch64-unknown-linux-gnu/release/gpio-cli
```

Copy the matching binary to the Pi. Because libgpiod is dynamically linked, the
Pi must have libgpiod 2.x installed (for Raspberry Pi OS: `libgpiod3` on newer
releases). Verify with `ldd ./gpio-cli` on the Pi.

Raspberry Pi 1 and Pi Zero use ARMv6 and are not covered by the `pi32` target.
For Pi 2 and newer running a 32-bit OS, use `pi32`; for a 64-bit OS, use `pi64`.
# rpi-gpio-cli
