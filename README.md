# 🌙 Satellite

轻量级 CloudFlare ImgBed 桌面客户端。支持 Windows / macOS。

## 功能

- **悬浮球入口**：始终置顶，拖放文件即可上传
- **上传历史**：本地 SQLite 存储，缩略图预览
- **快速复制**：上传完自动复制链接到剪贴板
- **一键跳转**：打开网页端管理后台
- **系统托盘**：最小化到托盘后台运行

## 技术栈

- [Tauri 2.0](https://tauri.app/) — 跨平台桌面框架
- [Rust](https://www.rust-lang.org/) — 上传逻辑、SQLite
- [Svelte 5](https://svelte.dev/) + [TailwindCSS](https://tailwindcss.com/) — 前端 UI

## 开发

### 前置要求

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) >= 18
- macOS: Xcode Command Line Tools
- Windows: MSVC Build Tools

### 启动

```bash
npm install
npm run tauri dev
```

### 构建

```bash
npm run tauri build
```

产物在 `src-tauri/target/release/bundle/`

## 配置

首次启动后，在「设置」标签页填写：

- **API Endpoint**: 你的 CloudFlare ImgBed 地址，如 `https://img.example.com`
- **Auth Token**: 如果图床启用了认证，填写对应 token

## 使用

1. 启动后会出现悬浮球（🌙）
2. 拖放文件到悬浮球 → 自动上传 → 链接复制到剪贴板
3. 点击悬浮球打开主窗口查看历史记录
4. 点击「管理后台」跳转网页端管理界面
