<h1 align="center" style="font-family: 'Source Han Serif', 'Source Han Serif CN', 'Source Han Serif SC', STSong, SimSun, serif; border: none; font-size: 48px; margin-bottom: 0;">
  <ruby>活<rt>huó</rt>字<rt>zì</rt></ruby><sup style="font-size: 12px;line-height:48px;vertical-align: 65%;"><i><small>Rust</small></i></sup>
</h1>
<h3 align="center" style="font-family: 'PingFang SC', 'Microsoft Yahei', sans-serif; font-style: normal; margin-top: 0; font-weight: 400;">
  <ruby>一个简单的中日韩文字排印引擎，为游戏富文本特别设计。<rt>A simple typography engine for CJK languages, especially designed for game rich-text.</rt></ruby>
</h3>
<h5 align="center">（功能尚在开发中）<a href="README_EN.md">[View English Version]</a></h5>

<hr>

<p align="center">
<a href="https://crates.io/crates/huozi" target="_blank" rel="noopener noreferrer"><img src="https://img.shields.io/crates/v/huozi.svg?style=flat-square" alt="" /></a>
<a href="https://docs.rs/huozi/latest/huozi/" target="_blank" rel="noopener noreferrer"><img alt="docs.rs" src="https://img.shields.io/docsrs/huozi?style=flat-square"></a>
<img src="https://img.shields.io/github/issues/icemic/huozi-rs.svg?style=flat-square" alt="" />
<a href="#许可"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg?style=flat-square" alt="" /></a>
</p>

## 总览

活字（Rust）是 [huozi.js](https://github.com/Icemic/huozi.js) 的新一代版本，不同于后者而使用 Rust 实现，其具有以下特点：

- 使用 SDF（Signed Distance Field）技术渲染字形
- 动态 SDF 字形生成和缓存，支持同时渲染最多 1024 个不同字形
- 支持多种排印效果，包括描边、阴影等
- 支持多种字体格式，包括 TTF、OTF
- 支持多种文字效果，包括下划线、删除线、颜色等（开发中）
- 输出为图片或者纹理，并提供顶点坐标和纹理坐标，方便与任意渲染引擎集成
- 实现[W3C 汉字排版需求](https://www.w3.org/TR/clreq/)，包括行内标点压缩、行内引号位置修正等（开发中）

## GUI 调试

运行 `cargo run --example render --release`，如下的 GUI 窗口：

![huozi gui debugger](snapshots/render.png)

## Roadmap

- [x] 字形生成和缓存
- [x] [WGPU](https://github.com/gfx-rs/wgpu) 渲染范例
- [x] 描边、阴影
- [x] 多种字体格式
- [ ] 多字体、字体 Fallback
- [x] 颜色
- [ ] 下划线、删除线
- [ ] 着重号、波浪下划线
- [x] 输出为图片或纹理，提供顶点坐标和纹理坐标
- [ ] W3C 汉字排版需求（标点悬挂、标点挤压）
- [ ] 连字
- [ ] 仿斜体、仿粗体
- [x] 支持 Windows、macOS、Linux、Android、iOS、Web (WebAssembly) 平台

尚在早期开发阶段（咕），尤其是排版功能高度缺失，欢迎任何 Issue 和 Pull Request！

Any translation is welcome!

## 使用

见 examples 目录。
