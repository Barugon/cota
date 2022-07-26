# Cross platform build (from Linux)

```bash
cargo install cross
```

- Create a `Dockerfile` in the project folder

```Dockerfile
FROM docker.io/library/ubuntu:20.04
RUN apt update
RUN DEBIAN_FRONTEND=noninteractive apt -y install tzdata
RUN apt -y install clang pkg-config libgtk-3-dev mingw-w64
```

- Create a `Cross.toml` file in the project folder

```toml
[target.x86_64-unknown-linux-gnu]
image = "docker.io/library/ubuntu:20.04"

[target.x86_64-pc-windows-gnu]
image = "docker.io/library/ubuntu:20.04"
```

- Build the docker image

```bash
sudo dnf install podman
podman build -t docker.io/library/ubuntu:20.04 .
```

- Build the project for Linux using cross

```bash
cross build --release --target=x86_64-unknown-linux-gnu
```

- Build the project for Windows using cross

```bash
cross build --release --target=x86_64-pc-windows-gnu
```
