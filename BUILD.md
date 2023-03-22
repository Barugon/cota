# Building CotA

## Linux

```bash
cargo build --release --target=x86_64-unknown-linux-gnu
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
