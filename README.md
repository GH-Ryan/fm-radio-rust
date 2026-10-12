# FM 电台

[![Store](https://img.shields.io/badge/Microsoft%20Store-0078D4?logo=microsoft&logoColor=white)](https://apps.microsoft.com/detail/9NCZJDND65GH)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

![电台列表](docs/store-assets/Screenshot-01-browse-zh.png)

Radio Browser 桌面客户端。Tauri 2 + Rust，自带流式解码，不依赖外部播放器。

Windows only。正在通过 Microsoft Store 审核。

## 特性

- 浏览、搜索 Radio Browser 全量电台目录
- 收藏，本地保存
- MP3 / AAC / AAC+ / FLAC / WAV 流式播放
- ICY 元数据（电台名、曲名）
- 中英双语，跟随系统语言
- 无账号、无遥测、无广告

## 构建

Windows 见 [BUILD-WINDOWS.md](BUILD-WINDOWS.md)。

前端要先构建 —— Tauri 编译期把 `dist/` 整个嵌进二进制：

```sh
cd frontend && npm install && npm run build && cd ..
cargo build --release --features custom-protocol
```

`custom-protocol` 不能省，否则运行时会去连 `localhost:1420` 然后退出。

## 测试

```sh
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## 格式

| 格式 | |
| --- | --- |
| MP3 / AAC / AAC+ / FLAC / WAV | ✅ |
| Opus | symphonia 0.5 不带 Opus 解码器 |
| Ogg Vorbis | 实测 Radio Browser 目录里仅 1.03% 的电台在用，且 Windows 桌面版系统解码器不含 Vorbis |

## License

MIT