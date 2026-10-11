# 隐私政策 / Privacy Policy

**在线版本 / Online version:** <https://gh-ryan.github.io/fm-radio-rust/privacy/>
（源文件 / Source: [`docs/privacy/index.html`](docs/privacy/index.html)）

---

## 摘要 / Summary

**中文：** FM Radio 不收集、不上传、不共享、不出售任何个人信息。应用没有账号体系，不要求登录，
不生成任何可用于识别你的标识，也不包含任何统计分析、广告或崩溃上报代码。开发者无法知道你是否
安装了本应用，也无法知道你播放过哪些电台。

**English:** FM Radio does not collect, upload, share, or sell any personal information. The app has
no account system, never asks you to sign in, generates no identifier that could recognise you, and
contains no analytics, advertising, or crash-reporting code. The developer cannot tell whether you
installed the app, or which stations you played.

## 保存在本地的内容 / Stored locally

| 内容 / Data | 位置 / Location |
| --- | --- |
| 收藏的电台 / Favourite stations | `favorites.json`（应用私有数据目录 / app private data folder） |
| 上次播放的电台 / Last played station | `last.json`（同上 / same） |
| 界面语言 / Interface language | WebView 本地存储，键名 `fmradio.locale` |

全部留在设备上，不上传、不跨设备同步。删除应用即全部删除。
All of it stays on the device: never uploaded, never synced. Uninstalling removes all of it.

## 网络请求 / Network requests

| 对象 / Party | 发送内容 / What is sent |
| --- | --- |
| Radio Browser (`*.api.radio-browser.info`) | 电台目录请求、**你输入的搜索词**。经 HTTPS。<br>Catalogue requests and **the search terms you type**, over HTTPS. |
| 电台运营方 / The broadcaster you play | 音频流连接。该运营方的日志可能记录 IP 与 User-Agent。<br>The audio stream connection. Their logs may record your IP and User-Agent. |
| 电台图标托管方 / Station image host | logo 图片请求。<br>Requests for station logo images. |

在你点击播放之前，应用不会向任何电台运营方发起请求。
The app makes no request to any broadcaster until you choose to play.

## 数据安全申报对应 / Data Safety declaration

在 Microsoft Store 的 Data Safety 问卷中，本应用对**所有**数据类型均选择
"此应用未收集用户数据 / This app does not collect user data"。

依据：无账号、无标识符、无遥测，且用户可产生的内容仅保存在本地、从未离开设备。

完整表述以在线版本为准。
The online version is authoritative.