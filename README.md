# vivin

面向个人的离线 Markdown 桌面应用，首发平台为 macOS。产品范围见[需求文档](docs/requirements.md)，界面方向见[概念图](docs/design/macos-ui-concept.png)和[交互补充说明](docs/design/macos-ui-spec.md)。

## 开发环境

- Node.js 22.12+、pnpm 10+
- Rust 工具链与 macOS Command Line Tools，安装方法见 [Tauri 前置要求](https://v2.tauri.app/start/prerequisites/)

```bash
pnpm install
pnpm tauri dev
```

只检查前端工程时，可运行 `pnpm dev`、`pnpm check` 和 `pnpm build`。原生部分在安装 Rust 后，进入 `src-tauri/` 运行 `cargo fmt --check`、`cargo check`，并生成、提交 `Cargo.lock`。

当前仓库已建立 Tauri 2 + Vue 3 + Vite + Tailwind CSS 4 的最小工程，并装入 Markdown 编辑依赖；项目文件管理和编辑器业务功能尚未接入。开发约定见[开发规范](docs/development.md)，编码代理须同时遵守仓库根目录的 [AGENTS.md](AGENTS.md)。
