# Building CotA

## Linux

```bash
cargo install cross
```

- Create a `Dockerfile` in the project folder

```Dockerfile
FROM docker.io/library/ubuntu:20.04
RUN DEBIAN_FRONTEND=noninteractive apt update && apt -y install tzdata && apt -y install clang pkg-config cmake libfontconfig-dev
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

```bash
rustup target add x86_64-pc-windows-gnu
sudo dnf install mingw64-gcc
```

- Build the project

```bash
cargo build --release --target=x86_64-pc-windows-gnu
```
