# Windows 构建说明

本项目仅支持 Windows，构建不使用 `make`。

## 一次构建

```powershell
cd frontend
npm.cmd install          # npm.cmd 是必须的，见下方"执行策略"
npm.cmd run build        # 产物到仓库根的 dist/，不是 frontend/dist

cd ..
cargo build --release --features custom-protocol
```

**前端必须先于 Rust 构建。** Tauri 在 Rust 编译期把 `dist/` 嵌进二进制，顺序反了会
打进旧前端，而且**不会有任何报错** —— 表现是改的前端代码"没生效"。

`--features custom-protocol` 不能省。Tauri 靠这个 feature 决定是读打包资源还是去连
`devUrl`，缺了会连 `http://localhost:1420` 然后退出。

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

### 打包脚本为什么要手工调 makepri

`tools/New-MsixPackage.ps1` 不使用 `winapp package`。winapp 会自己生成 `priconfig.xml`，
用 `startIndexAt=".\pri.resfiles"` 一个**文件清单**限定索引范围，而那份清单只列了
`Assets\`。清单驱动的索引会丢弃 `Strings\` 里的**逐语言候选** —— 所有字符串资源塌缩成
配置里的默认语言。包照样打得出来、校验也过，只是 `ms-resource:AppName` 解析不到任何
东西，开始菜单回退成原始标识符。

所以脚本手工组装暂存目录，用 `tools/priconfig.xml` 调 `makepri`，再 `makeappx` 打包、
`signtool` 签名。配置里有三条硬约束：

1. **`folder` 和 `resw` 两个 indexer 必须并存。** `folder` 遍历目录并从文件夹名推导语言
   限定符（`Strings\zh-CN\`），`resw` 解析 `.resw` 文件。缺 `resw` 时 `.resw` 会被当成
   普通 Path 资源，`ms-resource:AppName` 解析成一个文件名而不是字符串。
2. **必须从项目根索引，不能走 `pri.resfiles` 清单**，否则多语言候选丢失。
3. **默认语言必须是 `zh-CN`。** Partner Center 用清单里的 DisplayName 校验是否为你已
   预留的产品名，它读的是默认候选。预留名是 `FM 电台`，默认写成 `en-US` 会解析出
   `FM Radio` 并被拒。默认只是回退值，英文系统仍会解析到 `en-US` 候选。

### 本地化包显示名

开始菜单、设置 → 应用里显示的名字来自 **包清单**，不是应用内的 i18n：

- `Strings\zh-CN\Resources.resw` → `FM 电台`
- `Strings\en-US\Resources.resw` → `FM Radio`
- 清单写 `ms-resource:AppName` / `ms-resource:AppDescription`

文件夹名必须和清单里 `<Resource Language="..."/>` 的值一致，否则查找会静默回退到默认。

**`.resw` 的 XML 注释里不能出现 `--`。** 出现了整个文件就无法解析，而 makepri 报的是
`PRI224: root node not found` —— 指向配置问题，和真实原因毫无关系，很难排查。

### `$targetnametoken$`

清单里 `Executable="$targetnametoken$.exe"` 是 winapp 的占位符，它在打包时替换成真实
文件名，**makeappx 不认**，直接用会报：

> The file name "$targetnametoken$.exe" ... doesn't exist in the package.

`New-MsixPackage.ps1` 在暂存副本上把它替换成 `fm-radio`。

### 版本号

每次提交的版本必须**严格高于**上一次，所以 CI 用 `-Version` 参数覆盖清单里的值
（只在暂存副本上改，仓库里的清单保持不动）。

### 证书

开发用自签证书在 `msix\dev-cert.pfx`，被 `.gitignore` 排除。

证书 Subject 必须等于清单里的 `Identity/@Publisher`，否则签名与包身份不一致。CI 里每次
现生成一张（`tools/New-MsixPackage.ps1` 支持 `-CertPath`），证书只用于承载签名 ——
商店会用发布者证书重新签名。

本地安装测试需要把证书装进 `LocalMachine\Root`，只装 `CurrentUser\Root` 不够，会报：

> 应用包或捆绑包中的签名的根证书必须是受信任的证书。

MSIX 部署走系统级包管理器，只查 `LocalMachine\Root`，首次装包需要提权一次：

```powershell
$pw = ConvertTo-SecureString 'password' -AsPlainText -Force
$cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2(
  '.\msix\dev-cert.pfx', $pw)
foreach ($loc in @('Root', 'TrustedPeople')) {
  $store = New-Object System.Security.Cryptography.X509Certificates.X509Store($loc, 'LocalMachine')
  $store.Open('ReadWrite'); $store.Add($cert); $store.Close()
}
```

## MSIX 下的实际表现

**渲染**：`tauri.conf.json` 设了 `additionalBrowserArgs` 关掉 GPU
（`--disable-gpu --disable-gpu-compositing --disable-features=WebGPU,Vulkan`）。装成 MSIX
后实测窗口渲染、列表滚动、音量滑块均正常。如果之后出现白屏或滚动卡顿，第一个要试的
就是把这几个参数去掉。

**`decorations: false`（无边框窗口）**：自绘标题栏在 MSIX 下没有额外问题，但用户用不了
系统的贴靠和"贴靠后隐藏其他窗口"。

**数据路径**：MSIX 下 `%APPDATA%` 被重定向到包私有目录

```
C:\Users\<user>\AppData\Local\Packages\RayDigital.FM_<hash>\RoamingAppData\
```

`src/store/mod.rs` 里的 `dirs::config_dir()` 会自动跟随重定向，代码不用改。**但直接运行
exe（不走包身份）时写入的是普通 `%APPDATA%\fmradio\`，与包内路径是两份数据** —— 两者
不互通，切换运行方式时收藏看起来会"消失"。