<h1 align="center" style="font-family: 'Source Han Serif', 'Source Han Serif CN', 'Source Han Serif SC', STSong, SimSun, serif; border: none; font-size: 48px; margin-bottom: 0;">
  <ruby>活<rt>huó</rt>字<rt>zì</rt></ruby><sup style="font-size: 12px;line-height:48px;vertical-align: 65%;"><i><small>Rust</small></i></sup>
</h1>
<h3 align="center" style="font-family: 'PingFang SC', 'Microsoft Yahei', sans-serif; font-style: normal; margin-top: 0; font-weight: 400;">
  A simple typography engine for CJK languages, especially designed for game rich-text.
</h3>
<h5 align="center">(Working in progress) <a href="README.md">[查看中文版本]</a></h5>

<hr>

<p align="center">
<a href="https://crates.io/crates/huozi" target="_blank" rel="noopener noreferrer"><img src="https://img.shields.io/crates/v/huozi.svg?style=flat-square" alt="" /></a>
<a href="https://docs.rs/huozi/latest/huozi/" target="_blank" rel="noopener noreferrer"><img alt="docs.rs" src="https://img.shields.io/docsrs/huozi?style=flat-square"></a>
<img src="https://img.shields.io/github/issues/icemic/huozi-rs.svg?style=flat-square" alt="" />
<a href="#license"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg?style=flat-square" alt="" /></a>
</p>

## Overview

Huozi (Rust) is a new generation version of [huozi.js](https://github.com/Icemic/huozi.js). Unlike the latter, it is implemented in Rust and has the following features:

- Renders glyphs using SDF (Signed Distance Field) technology
- Dynamic SDF glyph generation and caching, supporting rendering up to 1024 different glyphs simultaneously
- Supports various typography effects, including stroke, shadow, etc.
- Supports multiple font formats, including TTF, OTF
- Supports various text effects, including underline, strikethrough, color, etc. (under development)
- Outputs as images or textures, and provides vertex coordinates and texture coordinates for easy integration with any rendering engine
- Implements [W3C Requirements for Chinese Text Layout](https://www.w3.org/TR/clreq/), including inline punctuation compression, inline quote position correction, etc. (under development)

## GUI Debugger

Run `cargo run --example render --release` to see the following GUI window:

![huozi gui debugger](snapshots/render.png)

## Roadmap

- [x] Glyph generation and caching
- [x] [WGPU](https://github.com/gfx-rs/wgpu) rendering example
- [x] Stroke, Shadow
- [x] Multiple font formats
- [ ] Multiple fonts, Font Fallback
- [x] Color
- [ ] Underline, Strikethrough
- [ ] Emphasis marks, Wavy underline
- [x] Output as image or texture, providing vertex and texture coordinates
- [ ] W3C Chinese Layout Requirements (Punctuation hanging, Punctuation squeezing)
- [ ] Ligatures
- [ ] Oblique, bold
- [x] Supports Windows, macOS, Linux, Android, iOS, Web (WebAssembly) platforms

Still in the early stages of development, especially with a high lack of typesetting features. Any Issues and Pull Requests are welcome!

Any translation is welcome!

## Usage

See the examples directory.
