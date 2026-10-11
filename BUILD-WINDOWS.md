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

走 `winapp`，不走 Tauri 自带的打包器 —— Tauri 只能出 EXE 和 MSI，而 MSI 上架需要
自购代码签名证书。

```powershell
# 1. 初始化 MSIX 清单和图标资产（只需一次）
winapp init

# 2. 生成开发用的自签证书（只需一次，之后复用同一个 pfx）
winapp cert generate

# 3. 打包 —— 必须用 --cert 复用已有证书
winapp package --cert com.fmradio.app_cert.pfx --cert-password password
```

**不要用 `--generate-cert` 反复打包。** 它每次都新造一张证书，而 MSIX 部署要求签名
证书在 `LocalMachine\Root` 受信任，换证书就得再提权装一次（UAC）。固定 pfx 只装一次。

产物是 `.msix`，可直接安装测试：

```powershell
Add-AppxPackage .\com.fmradio.app_0.1.0.0_x64.msix
```

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