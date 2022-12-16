# Companion of the Avatar

[![Dependency status](https://deps.rs/repo/github/Barugon/cota/status.svg)](https://deps.rs/repo/github/Barugon/cota)

CotA, a companion application for [Shroud of the Avatar](https://www.shroudoftheavatar.com), is written in 100% Rust using [egui](https://github.com/emilk/egui) (also 100% Rust) for it's user interface.

**Click [here](https://github.com/Barugon/cota/releases) for binaries**

> **Linux Note**: If you're using Wayland and you find the app's title bar disagreeable then set this environment variable before running: `WINIT_UNIX_BACKEND=x11`

<!-- intentional spacing -->

> **Windows note**: You might need to right click cota.exe (once unzipped), select `Properties` and then check `Unblock`.

<!-- intentional spacing -->

> **Mac note**: I no longer provide a Mac build due to Apple's licensing. However, building should be pretty easy — install [rust](https://www.rust-lang.org/tools/install), clone this repository and then build it using `cargo build --release`. You will probably also need to install Apple's [Xcode](https://developer.apple.com/download/all/?q=xcode).

## Features

### Display stats recorded to chat-logs via the `/stats` command

- Press F5 to refresh the display after typing `/stats` in-game
- Press Ctrl+R to get a list of effective resists
- Press Ctrl+F to filter the stats

![screenshot](https://a4.pbase.com/o12/09/605909/1/164136608.rcq0amhQ.Screenshotfrom20220710144634.png)

### Search chat Logs

Using either straight text or regular expressions.

![screenshot](https://a4.pbase.com/o12/09/605909/1/172748130.7CtBnycN.Screenshotfrom20220710145444.png)

### Portal and Cabalist chronometer

![screenshot](https://a4.pbase.com/o12/09/605909/1/166622004.99jNUqv1.Screenshotfrom20220702105817.png)

### Experience needed calculator

![screenshot](https://a4.pbase.com/o12/09/605909/1/169657368.6LSuf2mo.Screenshotfrom20220707123539.png)

### Offline save-game editor

![screenshot](https://a4.pbase.com/o12/09/605909/1/170775639.JD7XD39u.Screenshotfrom20221216144706.png)

![screenshot](https://a4.pbase.com/o12/09/605909/1/173233692.lKzg5kgV.Screenshotfrom20221216144200.png)
