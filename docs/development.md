# 开发规范与工程边界

## 当前基础

仓库使用 Tauri 2、Vue 3、TypeScript、Vite、Tailwind CSS 4、Oxlint 和 Oxfmt，包管理器为 pnpm。`src/` 是 WebView 界面，`src-tauri/` 是原生进程。项目服务、编辑器与本机 MCP 接口已接入。

遵循仓库根目录的 `AGENTS.md`：2 空格、严格类型、职责拆分、最小范围验证和 Conventional Commits。不要引入不适用于单应用项目的 monorepo、后台业务接口或并行的 ESLint/Prettier 配置。

## 目录职责

```text
src/
  App.vue          应用壳、项目与多栏交互
  components/      标题栏与两种 Markdown 编辑器
  lib/             标注定位、Tauri 命令类型与适配
src-tauri/src/
  project.rs       项目文件、资源、图片与草稿服务
  metadata.rs      可复制的项目结构化数据
  history.rs       AI 修改记录与冲突安全的撤销
  ai.rs            带令牌的本机 MCP 服务
docs/              开发说明
```

目录按功能真正出现时再建立。Vue 组件只处理交互和展示；项目文件的边界、版本检查、锁定、原子写入与撤销记录应由原生侧统一保证。前端请求和 Rust 返回值要使用稳定、可序列化的类型，错误码与用户可见提示分开。

## Markdown 编辑与渲染

- 使用 [Milkdown Crepe](https://milkdown.dev/docs/guide/using-crepe) 编辑排版视图，使用 [CodeMirror 6 Markdown](https://codemirror.net/docs/ref/#lang-markdown) 编辑原文视图。Milkdown 官方文档列有列表、待办项、代码块、图片与表格编辑能力。
- `.md` 文件是正文的唯一持久化来源。两种视图通过同一文档版本同步；切换视图前检查编辑器尚未保存的改动。双栏打开同一文件时共用文档内容，栏位独立保存选区、滚动位置和视图模式。
- 排版视图挂载时进行 Markdown 往返检查，若库会改写原文则退回原文视图。继续扩展语法时，用覆盖标题、段落、强调、链接、列表、待办项、引用、代码块、表格、图片及未知语法的样例做“读取→编辑→保存→再读取”验证。
- 用户文档按不可信内容处理。不要直接把 Markdown 中的原始 HTML 注入 `v-html`；若未来提供独立 HTML 预览，必须经过明确的 HTML 净化和安全链接处理。图片仅解析到当前项目允许的相对路径。
- 术语、标注、固定、锁定与 AI 修改记录写入项目内的结构化数据，不进入 `.md`。外部程序改动文件后先处理版本冲突，不能让编辑器自动保存覆盖磁盘内容。

## Tauri 边界

- 只暴露与业务动作对应的命令，不暴露任意路径读写。项目路径在原生侧规范化并核对真实位置，防止 `..` 或符号链接逃出项目根目录。
- 写操作应使用临时文件与原子替换或等效机制，并提供清晰失败结果。批量操作避免成功一半；AI 写入留下可查看和可撤销的项目内记录。
- 本机 AI 控制接口与前端共享项目服务和锁定规则，不通过另一套写入路径绕开校验。MCP 服务只监听本机，采用随机令牌；令牌路径由应用配置目录确定。
- Tauri capability 按功能逐项增加。项目数据随文件夹移动，不在项目元数据里保存当前机器的绝对路径。窗口布局等个人状态可放应用数据目录。
- 发布构建使用 Tauri CSP 限制 WebView 资源来源；开发模式暂沿用 Vite 默认连接。接入图片、字体或插件时按实际资源收紧或扩展 CSP，不把生产配置改回 `null`。

## 日常命令

```bash
pnpm install
pnpm tauri dev
pnpm check
pnpm build
pnpm format
```

Rust 工具链安装后，在 `src-tauri/` 运行 `cargo fmt --check`、`cargo check` 和 `cargo test`。提交前优先执行受影响范围的检查，再运行 `pnpm check`。依赖更新应保留 `pnpm-lock.yaml` 和 `src-tauri/Cargo.lock`，不要混用 npm 或 yarn 安装。桌面构建运行 `pnpm tauri build`。
