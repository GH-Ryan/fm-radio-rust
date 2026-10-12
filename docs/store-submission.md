# Microsoft Store 提交材料

| | |
| --- | --- |
| Store ID | `9NCZJDND65GH` |
| 包标识 / Package identity | `RayDigital.FM` |
| 发布者 DN / Publisher | `CN=3795FA3D-F467-470E-B67A-65A8BC923C67` |
| 发布者显示名 / Publisher display name | **Ray Digital** |
| Seller ID | `96541170` |
| 隐私政策 / Privacy policy | `https://gh-ryan.github.io/fm-radio-rust/privacy/` |
| Store 链接 | `https://apps.microsoft.com/detail/9NCZJDND65GH` |

前三项由 Partner Center 分配，可在产品的 **Product identity** 页查到，且必须与
`Package.appxmanifest` 里的 `Identity` 完全一致，否则包校验失败。

素材在 `store-assets/`。截图分中英两套（`-zh` / `-en`），分别上传到对应语言的 listing。

## 0. 预览

**Screenshots**（1366×768，上限 10 张，最少 1 张）

| | |
| --- | --- |
| ![浏览电台目录](store-assets/Screenshot-01-browse-zh.png) | ![正在播放](store-assets/Screenshot-02-playing-zh.png) |
| 1. 浏览 | 2. 正在播放 |

**Logos**

| 300×300 tile | 310×310 | 150×150 | Splash 620×300 |
| --- | --- | --- | --- |
| ![](store-assets/StoreLogo-300x300.png) | ![](store-assets/Square310x310Logo.png) | ![](store-assets/Square150x150Logo.png) | ![](store-assets/SplashScreen.png) |

> 上图仅供参考尺寸比例，实际提交的是这些文件本身，不是拼图。

**上传商店 vs 打进安装包** —— 目录里有两类，用途不同：

| 上传到 Store listing | 只打进 MSIX 包（Windows 从包内读取） |
| --- | --- |
| `Screenshot-*.png` | `Square44x44Logo.png`（任务栏） |
| `StoreLogo-300x300.png` | `Square71x71Logo.png` |
| `Square310x310Logo.png`（可选，覆盖包内） | `Square150x150Logo.png` |
| | `StoreLogo.png` |
| | `LockScreenLogo.png` |
| | `Wide310x150Logo.png` |
| | `SplashScreen.png` |

---

## 1. 应用名称 / App name

| 语言 | 名称 |
| --- | --- |
| zh-CN | FM 电台 |
| en-US | FM Radio |

## 2. 描述 / Description

### zh-CN

全球网络电台，点开即听。

· 数万个电台，可按台名或国家搜索
· 直接流式播放，无需下载，不依赖外部播放器
· 收藏常听的电台，下次打开还在
· 每张卡片显示台标、所在国家、编码格式和信号强度
· 免费，无广告，无内购

电台目录来自 Radio Browser 开放平台。

### en-US

Internet radio from around the world. Click and listen.

· Tens of thousands of stations, searchable by name or country
· Streams directly — nothing to download, no external player
· Save favourites for next time
· Every card shows the logo, country, codec, and signal strength
· Free. No ads. No in-app purchases.

Station directory provided by Radio Browser.

## 3. 截图说明 / Screenshot captions

上限 200 字符 / 200 characters max.

| # | 图片 | zh-CN | en-US |
| --- | --- | --- | --- |
| 1 | ![](store-assets/Screenshot-01-browse-zh.png) | 浏览电台目录，每张卡片显示台标、所在国家、编码格式和信号强度。 | Browsing the station directory. Every card shows the logo, country, codec, and signal strength. |
| 2 | ![](store-assets/Screenshot-02-playing-zh.png) | 正在播放，底部播放条显示电台名与采样率，可收藏、暂停、调节音量。 | Now playing. The player bar shows the station name and sample rate, with favourite, pause, and volume. |

## 4. 关键词 / Keywords

- zh-CN：`电台,广播,收音机,网络电台,在线广播,音乐,音乐播放器,Radio Browser`
- en-US：`radio,broadcast,internet radio,online radio,music,music player,stream,Radio Browser`

## 5. 其他字段 / Other fields

| 字段 | 值 |
| --- | --- |
| Category | Music（若下拉无此项，选 closest 的 Media / Entertainment） |
| Pricing | Free |
| Markets | 全部 / All |
| Audience | Public |
| Discoverability | 可在 Store 中被发现 |
| Website | 留空 —— 官方标注 Not required |
| Support contact | 留空 —— 仅 Xbox 上架时必填 |
| Age ratings | 逐题作答，见下 |

## 6. 年龄分级 / Age ratings

IARC 问卷分两步。第一步选 App Type，三选一：

| 选项 | 适用 |
| --- | --- |
| Game | 游戏或博彩类 |
| Social or Communication | 主要用途是社交或通讯 |
| **All Other App Types** | **本应用选这项** |

选定后才会渲染后续题目，全部选最低档：

| 题 | 答 |
| --- | --- |
| Downloaded App —— 包内是否含分级相关内容 | No |
| User Content Sharing —— 用户间能否交换内容 | No |
| Online Content —— 是否展示或推广非初始下载内的内容 | No |
| Promotion or Sale of Age-Restricted Products | No |
| 是否与他人共享用户精确位置 | No |
| 是否允许购买数字商品 | No |
| 是否含现金奖励、礼品卡、play-to-earn、加密货币或 NFT | No |
| 是否为浏览器或搜索引擎 | No |
| 是否为新闻或教育类产品 | No |
| 是否使用评级机构直接颁发的评级 / 是否通过实体介质分发 | No |

另有一个独立选项「I'm ready to complete the IARC questionnaire」需选中，否则题目不渲染。

---

## 提交状态

Submission 1 于 2026-10-11 提交，2026-10-12 通过认证并发布，公开链接
<https://apps.microsoft.com/detail/9NCZJDND65GH> 可访问。

包校验只有一条 `runFullTrust` 警告，属预期：应用需要 `Windows.FullTrustApplication`
入口点才能运行，这是桌面应用在 MSIX 下的正常形态。

后续更新走 CI：`.github/workflows/release.yml`，触发方式为推送 `v*` 标签。在下面的配置完成
之前，更新需在 Partner Center 手动上传包。

---

## 7. 启用自动更新（Entra 应用）

尚未配置。完成后推送 `v*` 标签即可自动构建、打包并提交商店。

工作流逻辑已就绪并对照官方文档校验过；缺的全是账号侧的配置，这三步都在浏览器里完成。

### 7.1 关联 Entra 租户

Partner Center → 设置 → 账户设置 → 租户管理。

若列表为空，选「关联现有 Azure AD 租户」或「创建新租户」。个人开发者账号同样需要这一步，
API 不接受 MSA 账号直接认证。

### 7.2 注册 Entra 应用

<https://entra.microsoft.com/> → 应用注册 → 新建注册。

| 项 | 填什么 |
| --- | --- |
| 名称 | 随意，例如 `fm-radio-rust CI` |
| 账户类型 | **仅组织目录中的账户** |
| 重定向 URI | 留空 |

注册后打开「证书和机密」→ 新建客户端机密 → **立即复制值**，页面关闭后不再显示。

三个 ID 各处一处，不要混用：

| 值 | 在哪找 | 用途 |
| --- | --- | --- |
| 租户 ID | 应用注册页「概述」→ 目录(租户) ID | `AZURE_AD_TENANT_ID` |
| 应用程序(客户端) ID | 「概述」→ 应用程序 ID | `AZURE_AD_APPLICATION_CLIENT_ID` |
| 客户端机密值 | 「证书和机密」→ 新建后显示一次 | `AZURE_AD_APPLICATION_SECRET` |
| Seller ID | Partner Center → 设置 → 开发者设置 → 卖方 ID | `SELLER_ID`（本项目为 `96541170`） |

### 7.3 在 Partner Center 授权

Partner Center → 设置 → 用户管理 → 添加用户。

| 项 | 填什么 |
| --- | --- |
| 电子邮件 | 填 7.2 的**应用程序(客户端) ID** —— 此处接受 ID 而非邮箱 |
| 类型 | Microsoft Entra 应用程序 |
| 角色 | **Manager** |

角色低于 Manager 会在 `msstore reconfigure` 阶段报权限错误。

### 7.4 配置仓库 Secrets

仓库 → Settings → Secrets and variables → Actions → New repository secret。

四个名称必须完全一致，工作流按名引用：

```
AZURE_AD_TENANT_ID
AZURE_AD_APPLICATION_CLIENT_ID
AZURE_AD_APPLICATION_SECRET
SELLER_ID
```

### 7.5 验证

```powershell
msstore reconfigure --tenantId <T> --sellerId 96541170 --clientId <C> --clientSecret <S>
msstore apps get 9NCZJDND65GH
```

`apps get` 能返回产品详情即配置正确。随后推送标签：

```bash
git tag v0.1.1.0
git push origin v0.1.1.0
```

### 7.6 注意事项

- **客户端机密会过期**（Entra 默认最长 24 个月）。过期后工作流在 reconfigure 步骤失败，
  重新建一个并更新 Secret 即可。
- **仅免费应用支持自动更新**，本应用免费。
- **新包版本号必须严格高于**已发布版本，否则商店拒绝。工作流从标签取版本号，
  标签写 `v0.1.1.0` → 包版本 `0.1.1.0`。
- **文案更新不在本工作流内**。`msstore publish` 会删除待提交草稿并基于上次发布的提交重建，
  已暂存的文案改动会被丢弃。要自动同步文案需另建 `metadata.json` + 独立工作流，
  且顺序必须是「先 publish 建草稿，再改 metadata」。