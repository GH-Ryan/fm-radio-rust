# FM 电台

Radio Browser 桌面客户端。Tauri + React，自带流式解码，不依赖外部播放器。

依赖（Arch）：

    webkit2gtk-4.1 gtk3 libsoup3 javascriptcoregtk-4.1 libappindicator-gtk3 librsvg

构建前端还需要 Node.js。

## 构建

    make build

产物 `target/release/fm-radio`。`build` 先跑 `npm run build`，前端在编译期嵌进二进
制，所以 `dist/` 里的旧文件会被静默打进包。`custom-protocol` feature 不能省，Tauri 靠
它决定读打包资源还是连 devUrl，缺了会连 `localhost:1420` 然后退出。

排版迭代用 `make dev-run`。release profile 开了 fat LTO 和 `codegen-units = 1`，改一行
CSS 也要两分钟。

    make size
    make clean

## 测试

    make test    # cargo test
    make lint    # cargo fmt --check + clippy -D warnings

## 格式

MP3 / AAC / Ogg Vorbis / FLAC / WAV。Opus 不支持，symphonia 0.5 没有 Opus 解码器。

## License

MIT
