# mp4-rs

[![shiguredo_mp4](https://img.shields.io/crates/v/shiguredo_mp4.svg)](https://crates.io/crates/shiguredo_mp4)
[![Documentation](https://docs.rs/shiguredo_mp4/badge.svg)](https://docs.rs/shiguredo_mp4)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## About Shiguredo's open-source software

We will not respond to pull requests or issues that have not been discussed on Discord. Additionally, Discord is only available in Japanese.

Please review <https://github.com/shiguredo/oss> before using this software.

## About Shiguredo's open-source software

Please read <https://github.com/shiguredo/oss> before using this software.

## Overview

A Rust library for reading and writing MP4 files.

## Features

- Zero dependency libraries
- Compatible with `no_std` environments
  - <https://docs.rust-embedded.org/book/intro/no-std.html>
- Sans I/O
  - <https://sans-io.readthedocs.io/index.html>
- Provides high-level APIs
- Provides C APIs
- Provides WebAssembly APIs
- Supports Windows / macOS / Linux
- Supports Opus / FLAC / AAC
- Supports VP8 / VP9 / AV1 / H.264 / H.265

## Roadmap

- Support for AV2
- Support for H.266 (VVC)
- Support for Fragmented MP4

## WebAssembly Sample Pages

We have prepared sample implementations using WebAssembly on GitHub Pages:

- [MP4 Dump](https://shiguredo.github.io/mp4-rs/examples/dump/)
- [MP4 Transcode](https://shiguredo.github.io/mp4-rs/examples/transcode/)

## Specifications

- ISO/IEC 14496-1
- ISO/IEC 14496-12
- ISO/IEC 14496-14
- ISO/IEC 14496-15
- [VP Codec ISO Media File Format Binding](https://www.webmproject.org/vp9/mp4/)
- [AV1 Codec ISO Media File Format Binding](https://aomediacodec.github.io/av1-isobmff/)
- [Encapsulation of Opus in ISO Base Media File Format](https://gitlab.xiph.org/xiph/opus/-/blob/main/doc/opus_in_isobmff.html)
- [Encapsulation of FLAC in ISO Base Media File Format](https://github.com/xiph/flac/blob/master/doc/isoflac.txt)

## License

Apache License 2.0

```text
Copyright 2024-2025, Takeru Ohta (Original Author)
Copyright 2024-2025, Shiguredo Inc.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
