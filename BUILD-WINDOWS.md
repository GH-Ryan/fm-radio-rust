# Windows 构建说明

`Makefile` 是 Linux 专用的 —— 它用 `stat -c`、`awk`、`size -A`、`rm -rf`，这些在
Windows 上都不存在，而且本机也没装 `make`。所以 Windows 上直接跑 cargo 和 npm，
不走 Makefile。

## 一次构建

```powershell
cd frontend
npm.cmd install          # npm.cmd 是必须的，见下方"执行策略"
npm.cmd run build        # 产物到仓库根的 dist/，不是 frontend/dist

cd ..
cargo build --release --features custom-protocol
```

`--features custom-protocol` 不能省。Tauri 靠这个 feature 决定是读打包资源还是去连
`devUrl`，缺了会连 `http://localhost:1420` 然后退出。Makefile 顶上那段注释讲的就是
这件事。

产物：`target\release\fm-radio.exe`

## 执行策略

PowerShell 默认禁止运行 `.ps1`，所以 `npm` 会被拦：

```
无法加载文件 npm.ps1，因为在此系统上禁止运行脚本
```

两个办法：

```powershell
# 办法一：显式调 .cmd（推荐，不改系统设置）
npm.cmd install

# 办法二：给当前用户放开策略
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
```

## 打 MSIX（Windows Store）

用仓库自带的脚本，**不要用 `winapp package`**：

```powershell
# 先构建前端，再构建 Rust —— Tauri 在 Rust 编译期把 dist/ 嵌进二进制，
# 顺序反了就会打进旧前端，且不会有任何报错。
cd frontend; npm.cmd install; npm.cmd run build; cd ..
cargo build --release --features custom-protocol

# 打包（组装暂存目录 → makepri → makeappx → signtool）
powershell -File tools\New-MsixPackage.ps1
```

产物在 `msix\build\`。

### 为什么不用 winapp package

winapp 打包时会自己生成 `priconfig.xml`，用 `startIndexAt=".\pri.resfiles"` 一个
**文件清单**限定索引范围，而那份清单只列了 `Assets\`。清单驱动的索引会丢弃
`Strings\` 里的**逐语言候选** —— 所有字符串资源塌缩成配置里的默认语言，其余翻译
静默消失。包照样打得出来、校验也过，只是 `ms-resource:AppName` 解析不到任何东西，
开始菜单回退成原始标识符。

`tools/New-MsixPackage.ps1` 改为手工组装并调用 `makepri` + `makeappx`，配置在
`tools/priconfig.xml`。

### makepri 的三个坑

1. **`folder` 和 `resw` 两个 indexer 必须并存。** `folder` 负责遍历目录并从文件夹名
   推导语言限定符（`Strings\zh-CN\`），`resw` 负责解析 `.resw`。缺 `resw` 时文件会被
   当成普通的 Path 资源，`ms-resource:AppName` 解析成一个文件名而不是字符串。
2. **不能用 `pri.resfiles` 清单**，必须从项目根索引，否则多语言候选丢失。
3. **`.resw` 的 XML 注释里不能出现 `--`。** 写了整个文件就解析失败，而 makepri 报的是
   `PRI224: root node not found` —— 指向配置问题，和真实原因毫无关系。

### `$targetnametoken$`

清单里 `Executable="$targetnametoken$.exe"` 是 winapp 的占位符，它在打包时替换成
真实文件名，**makeappx 不认**，直接用会报：

> The file name "$targetnametoken$.exe" ... doesn't exist in the package.

`New-MsixPackage.ps1` 在暂存副本上把它替换成 `fm-radio`。

### 本地化包显示名

开始菜单、设置 → 应用里显示的名字来自 **包清单**，不是应用内的 i18n：

- `Strings\zh-CN\Resources.resw` → `FM 电台`
- `Strings\en-US\Resources.resw` → `FM Radio`
- 清单写 `ms-resource:AppName` / `ms-resource:AppDescription`

文件夹名必须和清单里 `<Resource Language="..."/>` 的值一致，否则查找会静默回退到默认。

### 证书信任只认 LocalMachine

装 `CurrentUser\Root` **不够**，会报 `0x800B0109`：

> 应用包或捆绑包中的签名的根证书必须是受信任的证书。

MSIX 部署走的是系统级包管理器，只查 `LocalMachine\Root`。首次装包需要提权一次：

```powershell
$pw = ConvertTo-SecureString 'password' -AsPlainText -Force
$cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2(
  '.\com.fmradio.app_cert.pfx', $pw)
foreach ($loc in @('Root', 'TrustedPeople')) {
  $store = New-Object System.Security.Cryptography.X509Certificates.X509Store($loc, 'LocalMachine')
  $store.Open('ReadWrite'); $store.Add($cert); $store.Close()
}
```

## 已知的 Windows 特有风险

**`--disable-gpu` 可能是问题**

`tauri.conf.json` 里有：

```json
"additionalBrowserArgs": "--disable-gpu --disable-gpu-compositing --disable-features=WebGPU,Vulkan"
```

这是为了绕开 WebKitGTK 在 Linux 上的渲染崩溃。Windows 用的是 WebView2，不是
WebKitGTK，这个开关不一定需要，而且关掉 GPU 可能导致 WebView2 渲染异常。

**装成 MSIX 后必须实测**：窗口渲染、列表滚动、音量滑块。如果发现白屏或滚动卡顿，
第一个要试的就是把这几个参数去掉。

**`decorations: false`（无边框窗口）**

自绘标题栏在 MSIX 下没有额外问题，但用户用不了系统的贴靠和"贴靠后隐藏其他窗口"。

**数据路径会变**

MSIX 下 `%APPDATA%` 被重定向到包私有目录：

```
C:\Users\<user>\AppData\Local\Packages\com.fmradio.app_<hash>\RoamingAppData\
```

`src/store/mod.rs` 里的 `dirs::config_dir()` 会自动跟随重定向，所以代码不用改。
但**现有用户的收藏数据不会自动迁移** —— 老文件还在原路径。这是上架前要考虑的
体验问题。