# 品牌素材

这里是图标的**唯一来源**，其他位置的位图都由它生成，不要手工编辑生成结果。

| 文件 | 说明 |
|------|------|
| `app-icon.svg` | 矢量源文件（设计稿） |
| `app-icon-1024.png` | 1024×1024 渲染，所有位图从这里重采样 |

## 重新生成

```powershell
.\tools\assets\Sync-Branding.ps1
```

会更新：

- `src-tauri/icons/`：Tauri 应用图标（`32x32.png`、`128x128.png`、`128x128@2x.png`、多尺寸 `icon.ico`）
- `msix/PackageAssets/`：打进 MSIX 的包图标（含中英资源所需的全部尺寸）
- `assets/Square300x300Logo.png`：商店 listing 专用，不打进包

## 注意

- 生成脚本会给圆角图标补上不透明背景：Windows 会自己套遮罩，保留透明角会在任务栏上变成暗角。
- `New-StoreAssets.ps1` 会对 1024×1024 逐像素采样，运行时间较长（数分钟），只在改图后执行。
- 素材不参与日常构建；`Build-Release.cmd` 不会重跑这一步。
