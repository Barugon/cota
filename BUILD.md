# Building CotA

## Linux

The purpose of this is to create an executable that will work across most distributions, even if they have an older version of glibc.

```bash
cargo install cross
```

- Create a `Dockerfile` in the project folder

```Dockerfile
FROM docker.io/library/ubuntu:20.04
RUN DEBIAN_FRONTEND=noninteractive apt update && apt -y install tzdata && apt -y install clang pkg-config cmake libfontconfig-dev libx11-xcb-dev libxcb-render-util0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

- Create a `Cross.toml` file in the project folder

```toml
[target.x86_64-unknown-linux-gnu]
image = "docker.io/library/ubuntu:20.04"
```

- Build the docker image

```bash
sudo dnf install podman
podman build -t docker.io/library/ubuntu:20.04 .
```

- Build the project

```bash
cross build --release --target=x86_64-unknown-linux-gnu
```

## Windows

Cross compiling to Windows is pretty straight forward.

```bash
rustup target add x86_64-pc-windows-gnu
sudo dnf install mingw64-gcc
```

- Build the project

```bash
cargo build --release --target=x86_64-pc-windows-gnu
```