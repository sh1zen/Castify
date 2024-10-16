# Castify

A simple cross-platform screen caster tool made in Rust

## ✨ Features
- FullScreen, ScreenCrop, SaveCapture
- Multi-monitor support
- Keybindings support
- Beautiful minimalistic UI

## 📥 Installation
- ### Cargo
  Install Rust and Run the following command
    ```
    cargo install --git https://github.com/sh1zen/Castify
    ```

## 📋 Requirements Run
- **Mac OS**
    - Grant Access to Accessibility API: Add `Castify` to **System Preferences > Security & Privacy > Privacy > Accessibility**
    - Maybe disable Firewall for the Caster


## 💻 Requirements Dev
- **Linux**
    - Install the following packages:
      - `libgtk-3-dev`, `libgdk-pixbuf2.0-dev`
      - `ibext-dev`, `libxrender-dev`
      - `libpango1.0-dev`
      - `libx11-dev`, `libxi-dev`, `libxtst-dev`
      - `libglib2.0-dev`, `libxi-dev`, `libxtst-dev`



### 🙌🏻 Thanks to
- [iced](https://github.com/iced-rs) community for their help
- [gstreamer](https://github.com/GStreamer/gstreamer) community for their help
- Other crate maintainers