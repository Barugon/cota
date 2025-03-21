# Companion of the Avatar

CotA, a companion application for [Shroud of the Avatar](https://www.shroudoftheavatar.com), is written in 100% Rust using [egui](https://github.com/emilk/egui) (also 100% Rust) for it's user interface.

**Click [here](https://github.com/Barugon/cota/releases) for binaries**

> **Note**: **Windows** - You might need to right click cota.exe (once unzipped), select `Properties` and then check `Unblock`.

<!-- intentional spacing -->

> **Note**: **Mac** - I no longer provide a Mac build due to Apple's licensing. However, building should be pretty easy — install [rust](https://www.rust-lang.org/tools/install), clone this repository and then build it using `cargo build --release`. You will probably also need to install Apple's [Xcode](https://developer.apple.com/download/all/?q=xcode).

## Building

In order to build CotA, you will need to install [Rust](https://www.rust-lang.org/). Once that's done, download the source, change directory to where the source resides on your system and enter `cargo build --release`. The executable will be in the `target/release` sub-folder.

## Features

### Portal and Cabalist chronometer

![screenshot](https://a4.pbase.com/o12/09/605909/1/166622004.HmnxMOOo.ScreenshotFrom20250311221031.png)

### Experience planner

![screenshot](https://a4.pbase.com/o12/09/605909/1/169657368.kKZqL4w3.Screenshotfrom20230415150803.png)

### Agriculture

Add timers that remind you to water/harvest your plants via desktop notifications.

![screenshot](https://a4.pbase.com/o12/09/605909/1/173475863.jbhPupmK.Screenshotfrom20230317234357.png)

### Offline save-game editor

> **Note**: once you store your changes then you must reload the save-game in Shroud of the Avatar from the main menu.

![screenshot](https://a4.pbase.com/o12/09/605909/1/170775639.MMl94QYP.Screenshotfrom20230317234519.png)

### Display stats recorded to chat-logs via the `/stats` command

- Press F5 to refresh the display after typing `/stats` in-game
- Press Ctrl+R to get a list of effective resists
- Press Ctrl+F to filter the stats
- Press Ctrl+L to search the chat logs
- Press Ctrl+D to tally DPS

![screenshot](https://a4.pbase.com/o12/09/605909/1/164136608.QBmjRKgr.Screenshotfrom20230317234632.png)
