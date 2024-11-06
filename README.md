# Castify

A simple cross-platform screen caster tool made in Rust

![ScreeShoot](./resources/screen.png)

## ✨ Features
- FullScreen, ScreenCrop, SaveCapture
- Multi-monitor support
- Keybindings support
- Beautiful minimalistic UI

## 📥 Installation
- ### Fulfill these [requirements](#requirements).
- ### Cargo
  Install Rust and Run the following command
    ```
    cargo install --git https://github.com/sh1zen/Castify
    ```

## 📋 Requirements Run
<a id="requirements"></a>
- **Windows**
    - Install [GStreamer](https://github.com/GStreamer/gstreamer)
- **Mac OS**
    - Install [GStreamer](https://github.com/GStreamer/gstreamer)
    - Grant Access to Accessibility API: Add `Castify` to **System Preferences > Security & Privacy > Privacy > Accessibility**
    - Maybe disable Firewall for the Caster
- **Linux**
    - Install [GStreamer](https://github.com/GStreamer/gstreamer)

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