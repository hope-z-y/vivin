<div align="center">
  <img src="public/vivin-logo.svg" alt="vivin" width="280">

  <p><strong>把 Markdown、资料和写作上下文留在自己的文件夹里。</strong></p>

  <p>一款面向 macOS 与 Windows 的本地优先 Markdown 桌面应用。</p>

  <p>
    <a href="https://github.com/hope-z-y/vivin/actions/workflows/desktop-build.yml"><img src="https://github.com/hope-z-y/vivin/actions/workflows/desktop-build.yml/badge.svg" alt="Desktop build"></a>
    <img src="https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white" alt="Tauri 2">
    <img src="https://img.shields.io/badge/Vue-3-42B883?logo=vuedotjs&logoColor=white" alt="Vue 3">
  </p>
</div>

## vivin 是什么

vivin 将一个普通文件夹作为写作项目。正文始终是可由其他编辑器打开的 `.md` 文件；图片、参考资料、术语、标注和 AI 修改记录也保存在项目文件夹中。复制或同步这个文件夹，就能带走项目内容。应用的语言、主题和最近项目等个人偏好保存在本机。

## 功能

- **专注写作：**在排版视图与 Markdown 原文视图之间切换；支持标题、列表、待办、引用、代码块、表格、图片和公式。遇到排版视图无法安全往返保存的 Markdown 时，自动切回原文视图。
- **并排阅读与编辑：**最多打开左右两栏，每栏可独立选择文档和视图；同一文档在双栏中共享内容。文档大纲可快速定位标题，大文档使用分段渲染。
- **整理项目：**创建、重命名、移动和删除文档或目录；固定常用条目，锁定文档或目录以防误改。目录锁定会作用于其下内容。
- **查找与批注：**搜索当前项目的 Markdown 文档；为选中文字添加待办、批注、疑问或错误标注，并通过侧栏和文档缩略条定位。
- **统一术语：**为项目术语添加解释，在正文中悬停查看；可以对单个出现位置关闭解释。
- **管理资料与图片：**在侧栏浏览 `resources/` 中的参考文件，并交由系统默认程序打开；粘贴或拖入图片后，图片会保存到项目的 `attachments/`，正文使用相对路径引用。
- **本机 AI 协作：**运行中的应用提供带令牌的本机 MCP 接口。AI 操作遵守项目规则、锁定状态和文档版本检查；成功的修改留下可查看、可撤销的记录。
- **界面偏好：**提供中文和英文界面、浅色和深色模式，以及最近项目切换。

## 开始使用

### 获取应用

目前没有签名的正式发行版。仓库的 [Desktop build 工作流](https://github.com/hope-z-y/vivin/actions/workflows/desktop-build.yml) 会构建 macOS Apple Silicon、macOS Intel 和 Windows x64 安装包，并将结果保存为 GitHub Actions artifacts；也可以按下文从源码构建。未签名的构建产物可能受到操作系统的安装限制。

### 创建项目

1. 打开 vivin，点击右上角 **+**，选择 **新建项目**，或用 **打开项目文件夹** 打开已有文件夹。
2. 在左侧新建 `.md` 文档或目录，开始写作。顶部可切换排版与原文视图、打开大纲或分栏。
3. 将参考文件放进项目的 `resources/`。图片可直接粘贴或拖进编辑器。
4. 需要项目规则时，在 **更多 → 编辑项目规则** 中编辑该项目的 `AGENTS.md`。

一个项目文件夹大致如下：

```text
我的项目/
├── AGENTS.md           # 当前项目的写作规则
├── 文章.md              # 普通 Markdown 文件
├── attachments/        # 从编辑器导入的图片
├── resources/          # 参考文件
└── .vivin/             # 固定、锁定、术语、标注和 AI 修改记录
```

项目根目录的 `AGENTS.md` 是该项目的写作规则；本源码仓库的 [`AGENTS.md`](AGENTS.md) 则是贡献者和编码代理的开发约定，两者用途不同。`resources/` 文件可浏览和打开，但不会被解析或纳入 Markdown 全文搜索。

### 连接本机 AI 客户端

vivin 运行时在 `http://127.0.0.1:38419/mcp` 提供 MCP Streamable HTTP 接口。首次运行会在应用配置目录生成 `bridge.json`，其中包含连接 URL 和令牌。通过 **更多 → Codex 连接信息** 可查看配置文件位置；客户端需在请求中发送 `Authorization: Bearer <token>`。请勿将令牌或配置文件提交到仓库。

AI 写入前应先调用 `project_context` 读取当前项目规则与规则版本；修改文档时还需提供当前文档版本。接口只处理已打开的项目，并执行路径边界、锁定和版本冲突检查。端口被占用时，编辑器仍可使用，但 MCP 服务需要在释放端口并重启应用后才能连接。

## 从源码运行

需要 Node.js 22.12+、pnpm 10+、Rust stable，以及对应平台的 [Tauri 2 前置环境](https://v2.tauri.app/start/prerequisites/)。

```bash
git clone https://github.com/hope-z-y/vivin.git
cd vivin
pnpm install --frozen-lockfile
pnpm tauri dev
```

检查与构建：

```bash
pnpm check          # 类型、Lint 和格式检查
pnpm build          # 前端构建
pnpm test:segments  # Markdown 分段测试
pnpm test:outline   # 大纲测试
cd src-tauri && cargo test
```

运行 `pnpm tauri build` 可按当前平台生成桌面包；macOS 可指定 `--bundles app,dmg`，Windows 可指定 `--bundles nsis`。工程采用 Vue 3、TypeScript、Vite、Tailwind CSS 4、Milkdown Crepe、CodeMirror 6 和 Tauri 2 / Rust。目录职责与开发约定见 [开发说明](docs/development.md)。

## 当前状态与参与贡献

项目仍处于早期阶段。macOS Apple Silicon 已进行本机验证；Windows 构建已配置在 CI 中，但尚未完成 Windows 实机验收。对外发行前还需要签名、公证及安装和升级测试。欢迎通过 [Issues](https://github.com/hope-z-y/vivin/issues) 反馈问题，或提交 Pull Request。修改前请阅读 [开发说明](docs/development.md) 和仓库 [开发约定](AGENTS.md)，并运行与改动相关的检查。

## 许可证

本项目采用 [MIT License](LICENSE)。
