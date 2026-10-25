# vivin

离线 Markdown 桌面应用，支持 macOS 和 Windows。项目就是一个本地文件夹，文档、图片、资源库、术语、标注与 AI 修改记录均随文件夹复制。功能范围见 [需求文档](docs/requirements.md)，交互见 [界面说明](docs/design/macos-ui-spec.md)。

## 使用

打开应用后点击右侧工具栏的“+”，选择已有文件夹或新建项目。点击顶部项目名称可切换最近项目；右侧还有中英文与明暗模式切换。左侧管理 Markdown 文档，并只读查看项目 `resources/` 文件夹中的资源数量和文件；编辑区可切换排版和原文视图，也可开启双栏。选中文字后，可在浮层工具栏直接添加待办、批注、疑问或错误标注。在编辑器中粘贴或拖入 PNG、JPEG、GIF、WebP 图片后，图片会复制到项目的 `attachments/`，正文写入相对路径。项目的 `resources/` 用于保存参考文件，不参与全文搜索。

顶部“更多”菜单可编辑项目规则、查看 AI 修改记录和 Codex 连接信息。项目规则保存在项目根目录的 `AGENTS.md`；`.vivin/` 保存结构化项目数据和 AI 修改记录。

## 开发与构建

需要 Node.js 22.12+、pnpm 10+、Rust stable，以及目标平台的 [Tauri 2 前置环境](https://v2.tauri.app/start/prerequisites/)。

```bash
pnpm install --frozen-lockfile
pnpm tauri dev
pnpm check
cd src-tauri && cargo test && cd ..
pnpm tauri build
```

macOS 上可用 `pnpm tauri build --bundles app,dmg` 生成 `.app` 与 `.dmg`；Windows 上可用 `pnpm tauri build --bundles nsis` 生成安装程序。`.github/workflows/desktop-build.yml` 配有 macOS Apple Silicon、Intel 和 Windows x64 构建，产物存为 GitHub Actions artifacts。未使用发行证书时，构建产物不会有正式开发者签名。

## 图标与功能演示项目

应用图标的可编辑源稿是 `public/vivin-icon.svg`，独立标志与横向字标分别是 `public/vivin-mark.svg`、`public/vivin-logo.svg`。透明背景上的深蓝 V 与亮蓝 I 组成 VI 字母标志。Tauri 的 macOS、Windows 图标位于 `src-tauri/icons/`；修改应用图标后可运行 `pnpm tauri icon public/vivin-icon.svg --output src-tauri/icons` 重新生成。

`examples/vivin-feature-tour/` 是可直接打开的 MOCK 演示项目。建议先复制到仓库外，再在 vivin 中点击右上角 `+` →“打开项目文件夹”选择复制后的文件夹，打开 `00-从这里开始.md` 跟随导览。它包含排版样例、图片附件、资源库、术语及出现位置例外、四类标注、固定与锁定、多层目录，以及一条可撤销的 AI 修改记录。双栏、搜索、文件操作、空状态、语言和主题切换等需要按导览文档亲手触发。改动演示文档的标注短语后，可运行 `node scripts/generate-feature-tour-data.mjs` 更新位置数据；`node scripts/generate-feature-tour-data.mjs --check` 只检查数据是否匹配。

## Codex 本机接口

应用运行期间在 `http://127.0.0.1:38419/mcp` 提供带令牌的 MCP Streamable HTTP 接口。首次运行生成应用配置目录下的 `bridge.json`，其中包含连接 URL 和令牌；在“更多 → Codex 连接信息”可查看配置文件路径。MCP 客户端需发送 `Authorization: Bearer <token>`。令牌仅保存在本机，不要提交到仓库。

Codex 写入前先调用 `project_context` 读取当前项目 `AGENTS.md` 及规则版本；文档写入还需传入当前文档版本。接口按项目隔离，检查锁定和版本冲突，并将成功的 AI 修改写入项目内可撤销的记录。

开发约定见 [开发规范](docs/development.md)，已验证范围和发布限制见 [验证记录](docs/verification.md)。编码代理还应遵守仓库根目录的 [AGENTS.md](AGENTS.md)。
