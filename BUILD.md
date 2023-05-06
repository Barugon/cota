# Building CotA

## Linux

```bash
cargo build --release --target=x86_64-unknown-linux-gnu
```

## Windows

Cross compiling to Windows is pretty straight forward.

> Note: this is only for cross compiling a Windows build from Linux. To build from Windows, you should only need to run `cargo build --release`.

```bash
rustup target add x86_64-pc-windows-gnu
sudo dnf install mingw64-gcc
```

- Build the project

```bash
cargo build --release --target=x86_64-pc-windows-gnu
```
