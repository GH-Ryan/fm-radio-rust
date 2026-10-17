<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="128" alt="FM 电台">
</p>

<h1 align="center">FM 电台</h1>

<p align="center">
  <a href="https://apps.microsoft.com/detail/9NCZJDND65GH"><img src="https://get.microsoft.com/images/en-us%20dark.svg" height="52" alt="Get it from Microsoft Store"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
</p>

<p align="center">
  浏览全球网络电台目录，点击即听。Tauri 2 + Rust + React，音频由内置解码器流式处理。
</p>

![电台列表](assets/Screenshot-01-browse-zh.png)

![正在播放](assets/Screenshot-02-playing-zh.png)

---

## 特性

- **浏览与搜索** —— Radio Browser 全量电台目录，可按台名、国家、标签、热门度浏览
- **流式播放** —— 直接播放，无需下载，不依赖系统解码器或外部播放器
- **收藏** —— 常听的电台存在本机，下次打开还在
- **电台信息** —— 每张卡片显示台标、所在国家、编码格式和信号强度
- **歌曲元数据** —— 解析 ICY 元数据，显示当前曲名；兼容 UTF-8 与 GB18030 编码的电台
- **随机播放** —— 从当前列表或收藏列表随机挑一个
- **贴靠支持** —— 无边框窗口，自绘标题栏，可用 Windows 贴靠
- **中英双语** —— 跟随系统语言，无需设置
- **无账号、无遥测、无广告**

## 下载

通过 Microsoft Store 获取（Windows 10 1809 / 11 及以上）：

<https://apps.microsoft.com/detail/9NCZJDND65GH>

其他安装方式见下方[从源码构建](#从源码构建)。

## 更新日志

当前版本：**v0.1.0.0**

完整更新日志：[docs/CHANGELOG.md](docs/CHANGELOG.md)

## 格式支持

解码由 [symphonia](https://docs.rs/symphonia) 完成，不依赖系统解码器。

| 格式 | 状态 |
| --- | --- |
| MP3 | ✅ |
| AAC / AAC+ | ✅ |
| FLAC | ✅ |
| WAV | ✅ |
| Ogg Vorbis | ✅ |
| Opus | ❌ 未启用 symphonia 的 Opus 解码器 |

> 电台实际使用什么格式由电台方决定。目录里标注的编码信息仅供参考，部分电台标注与实际流不一致。

## 系统要求

- Windows 10 1809 或更高
- 播放 MP3 / AAC 无需任何额外组件 —— 解码在应用内完成
- Microsoft Store 版本会自动安装 WebView2 运行时

## 从源码构建

### 前置条件

- [Rust](https://rustup.rs/) stable（edition 2021）
- [Node.js](https://nodejs.org/) 18 或更高
- Visual Studio Build Tools 的「使用 C++ 的桌面开发」工作负载
- Windows SDK（Windows 桌面构建需要；打包 MSIX 时还需其中的 `makepri` / `makeappx`）

### 构建

**前端必须先于 Rust 构建。** Tauri 在编译期把 `dist/` 整个嵌入二进制，顺序反了会打进旧前端，而且不会报错。

```sh
npm install && npm run build
cd src-tauri && cargo build --release --features custom-protocol
```

产物：`src-tauri\target\release\fm-radio.exe`

`--features custom-protocol` 不能省 —— 它决定 Tauri 读取打包资源还是去连开发服务器。

开发时用 `npm run tauri dev`，会同时启动前端开发服务器和应用，改动即时生效。

### 测试

在 `src-tauri/` 下执行：

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### 打包 MSIX

包源在 `msix/`（清单、图标、资源字符串），脚本在 `tools/`。发新版的完整步骤：

```powershell
npm install && npm run build
cd src-tauri && cargo build --release --features custom-protocol && cd ..
.\tools\New-MsixPackage.ps1 -Version "0.1.0.0"
```

产物在 `msix\build\`，上传到 Partner Center 的「管理程序包」。

脚本手工组装暂存目录并调用 `makepri` / `makeappx` / `signtool`，而不用 `winapp package`：
winapp 生成的 `priconfig.xml` 用一份只列 `Assets\` 的清单限定索引范围，会丢掉 `Strings\` 里的
逐语言候选，`ms-resource:AppName` 因此解析不到东西，开始菜单回退成包标识符。

## 技术栈

| 层 | 技术 | 用途 |
| --- | --- | --- |
| 界面 | React 18 + TypeScript + Vite | 组件与状态，热重载开发 |
| 容器 | Tauri 2 | 原生窗口、WebView2 宿主、进程间调用 |
| 后端 | Rust（edition 2021） | 目录请求、播放控制、本地存储 |
| 音频 | symphonia 解码 + rodio 输出 | 流式解码与声卡输出，不依赖系统解码器 |
| 网络 | reqwest（rustls） | Radio Browser API 与电台音频流，纯 Rust TLS |

## 隐私

本应用不收集任何数据：无账号、无标识符、无遥测、无广告。收藏与设置保存在本机。

点击播放前不会向任何电台发起请求；开始播放后仅向所选电台的服务器请求音频流，并向 Radio Browser 的公开 API 请求目录数据。

完整说明见 [docs/privacy](docs/privacy/index.html)。

## 商店发布信息

供维护者发布时参考。

### 身份

| 项 | 值 |
| --- | --- |
| Store ID | `9NCZJDND65GH` |
| 包标识 / Package identity | `RayDigital.FM` |
| 发布者 DN / Publisher | `CN=3795FA3D-F467-470E-B67A-65A8BC923C67` |
| 发布者显示名 / Publisher display name | Ray Digital |
| Seller ID | `96541170` |
| 隐私政策 | <https://gh-ryan.github.io/fm-radio-rust/privacy/> |

前三项由 Partner Center 分配，必须与 `msix/Package.appxmanifest` 里的 `Identity` 完全一致，
否则包校验失败。

### 素材

截图分中英两套（`assets/Screenshot-0?-*-zh.png` / `-en.png`），上传到对应语言的 listing，
两套不能混用。其余图标的归属看目录：`assets/` 上传商店，`msix/Assets/` 打进包，
`src-tauri/icons/` 是 Tauri 应用图标（`icon.ico` 编译进 exe 决定窗口与任务栏图标）。

全部素材由 `tools/New-StoreAssets.ps1` 从 `src-tauri/icons/icon-1024.png` 生成，
`Square310x310Logo.png` 是唯一同时用于包和 listing 的尺寸。

### 文案

| 语言 | 名称 |
| --- | --- |
| zh-CN | FM 电台 |
| en-US | FM Radio |

**zh-CN 描述**

```
全球网络电台，点开即听。

· 数万个电台，可按台名或国家搜索
· 直接流式播放，无需下载，不依赖外部播放器
· 收藏常听的电台，下次打开还在
· 每张卡片显示台标、所在国家、编码格式和信号强度
· 免费，无广告，无内购

电台目录来自 Radio Browser 开放平台。
```

**en-US 描述**

```
Internet radio from around the world. Click and listen.

· Tens of thousands of stations, searchable by name or country
· Streams directly — nothing to download, no external player
· Save favourites for next time
· Every card shows the logo, country, codec, and signal strength
· Free. No ads. No in-app purchases.

Station directory provided by Radio Browser.
```

**截图说明**（上限 200 字符）

| # | zh-CN | en-US |
| --- | --- | --- |
| 1 | 浏览电台目录，每张卡片显示台标、所在国家、编码格式和信号强度。 | Browsing the station directory. Every card shows the logo, country, codec, and signal strength. |
| 2 | 正在播放，底部播放条显示电台名与采样率，可收藏、暂停、调节音量。 | Now playing. The player bar shows the station name and sample rate, with favourite, pause, and volume. |

**关键词**

- zh-CN：`电台,广播,收音机,网络电台,在线广播,音乐,音乐播放器,Radio Browser`
- en-US：`radio,broadcast,internet radio,online radio,music,music player,stream,Radio Browser`

## License

[MIT](LICENSE)