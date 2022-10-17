![CI](https://github.com/pyaillet/esp-idf-ble/workflows/Continuous%20integration/badge.svg)
![MIT/Apache-2.0 licensed](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)

# twatch-idf-rs

TTGO [T-Watch 2020 v1](http://www.lilygo.cn/prod_view.aspx?TypeId=50053&Id=1380&FId=t3:50053:3) Rust firmware.

## Preview

[demo.webm](https://user-images.githubusercontent.com/11957179/196264757-b4c116e9-1d47-490b-8251-05fd6c138986.webm)


## Description

This project is a Work in Progress of a rust firmware for the T-Watch-v1 from Lilygo.

What's working ?

- [x] Power Management Unit - using my own driver for [AXP202](https://github.com/pyaillet/axp20x-rs)
  - [x] Power button
  - [x] Battery level
  - [ ] Plugged in status - Not tested
  - [ ] Deep sleep
- [x] Screen - using [mipidsi crate](https://github.com/almindor/mipidsi)
  - [x] Backlight settings
- [x] Touchscreen - using my own driver for [FT6x36](https://github.com/pyaillet/ft6x36-rs)
- [x] Accelerometer - using my own driver of [BMA423](https://github.com/pyaillet/bma423-rs/)
  - [x] X/Y/Z axis sensors
  - [ ] Activity recognition
  - [ ] Step counter
- [ ] I2S Speaker
- [x] WiFi should work, but not used right nown
- [ ] BLE - WIP [here](https://github.com/pyaillet/esp-idf-ble)
- [x] Vibration with the included motor
- [x] Clock - using [PCF8563 realtime clock driver](https://github.com/nebelgrau77/pcf8563-rs)
  - [x] Time
  - [ ] Alarms - Not tested

## What's included

This project is a tech demo. The firmware comes with 5 tiles demonstrating some features:

- [Hello world](./src/tiles/hello.rs): only displays text
- [Light](./src/tiles/light.rs): adjust brightness of the screen backlight
- [Motor](./src/tiles/motor.rs): demonstrate the vibrator
- [Time](./src/tiles/time.rs): Shows Realtime clock, battery level, accelerometer and swipe gestures
- [Sleep](./src/tiles/sleep.rs): Disable screen and backlight when button is pressed

## Credits

Many things from this project are inspired by the [rust-esp32-std-demo](https://github.com/ivmarkov/rust-esp32-std-demo).
Kudos to the people on [#esp-rs:matrix.org](https://matrix.to/#/#esp-rs:matrix.org) for their help.

## How to use?

Refer to [this repo](https://github.com/esp-rs/rust-build) to install the custom Rust ESP toolchain. You should also install [cargo espflash](https://github.com/esp-rs/espflash) to ease the use of this project.

Then you can launch the following command to compile one of the example, flash it to your device and monitor the ESP32 serial:

`cargo espflash --monitor --speed 921600 <device>`

