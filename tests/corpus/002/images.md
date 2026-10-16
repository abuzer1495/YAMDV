# Images

Inline badges: [![build](https://img.shields.io/badge/build-passing-green.svg)](https://example.org) ![license](https://img.shields.io/badge/license-MIT-blue.svg) and a local ![logo](typical-assets/one.png) in a line.

A remote block image:

![Remote diagram](https://example.org/diagram.png)

Local relative: ![First asset](typical-assets/one.png)

Parent folder: ![](../002/typical-assets/two.png)

Data URI: ![dot](data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==)

Missing: ![not here](missing/nothing.png)

UNC path (blocked before any file access): ![share](//server/share/pic.png)

WebDAV (blocked): ![dav](\\host@SSL\DavWWWRoot\pic.png)

SVG (unsupported until Phase 010): ![vector](drawing.svg)

Obsidian size: ![[one.png|300]]

<p align="center"><img src="typical-assets/two.png" width="200" alt="Centered HTML image"></p>
