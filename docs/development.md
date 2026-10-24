# 开发规范与工程边界

## 当前基础

仓库使用 Tauri 2、Vue 3、TypeScript、Vite、Tailwind CSS 4、Oxlint 和 Oxfmt，包管理器为 pnpm。`src/` 是 WebView 界面，`src-tauri/` 是原生进程。此阶段建立了可构建的空工作区和依赖，尚未实现项目文件操作、Markdown 编辑器或本机 AI 接口。

参考 `iihua_vben` 的 2 空格、严格类型、职责拆分、最小范围验证和 Conventional Commits 做法；该项目的 monorepo、后台业务接口、权限体系和 ESLint/Prettier 配置不适用于 vivin。

## 目录职责

```text
src/
  features/        按项目、编辑器、搜索、术语、标注等功能组织
  components/      跨功能复用的界面组件
  lib/             纯函数、类型和 Tauri 命令适配层
src-tauri/src/     原生入口、项目服务和命令
docs/              产品需求、界面说明和开发决策
```

目录按功能真正出现时再建立。Vue 组件只处理交互和展示；项目文件的边界、版本检查、锁定、原子写入与撤销记录应由原生侧统一保证。前端请求和 Rust 返回值要使用稳定、可序列化的类型，错误码与用户可见提示分开。

## Markdown 编辑与渲染

- 已安装 [Milkdown Crepe](https://milkdown.dev/docs/guide/using-crepe) 及 Vue 集成，用于排版视图；已安装 [CodeMirror 6 Markdown](https://codemirror.net/docs/ref/#lang-markdown)，用于原文视图。Milkdown 官方文档列有列表、待办项、代码块、图片与表格编辑能力。
- `.md` 文件是正文的唯一持久化来源。两种视图通过同一文档版本同步；切换视图前检查编辑器尚未保存的改动。双栏打开同一文件时共用文档内容，栏位独立保存选区、滚动位置和视图模式。
- 首个编辑器功能完成前，用覆盖标题、段落、强调、链接、列表、待办项、引用、代码块、表格、图片以及未知语法的样例做“读取→编辑→保存→再读取”验证。库不支持的 Markdown 不得被静默删除或改写；必要时保留原文模式并阻止有损转换。
- 用户文档按不可信内容处理。不要直接把 Markdown 中的原始 HTML 注入 `v-html`；若未来提供独立 HTML 预览，必须经过明确的 HTML 净化和安全链接处理。图片仅解析到当前项目允许的相对路径。
- 术语、标注、固定、锁定与 AI 修改记录写入项目内的结构化数据，不进入 `.md`。外部程序改动文件后先处理版本冲突，不能让编辑器自动保存覆盖磁盘内容。

## Tauri 边界

- 只暴露与业务动作对应的命令，不暴露任意路径读写。项目路径在原生侧规范化并核对真实位置，防止 `..` 或符号链接逃出项目根目录。
- 写操作应使用临时文件与原子替换或等效机制，并提供清晰失败结果。批量操作避免成功一半；AI 写入留下可查看和可撤销的项目内记录。
- 本机 AI 控制接口与前端共享项目服务和锁定规则，不通过另一套写入路径绕开校验。只监听本机，具体协议和授权方式在接口开发时确定。
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

Rust 工具链安装后，在 `src-tauri/` 运行 `cargo fmt --check` 和 `cargo check`，并生成、提交 `Cargo.lock`。提交前优先执行受影响范围的检查，再运行 `pnpm check`。依赖更新应保留 `pnpm-lock.yaml`，不要混用 npm 或 yarn 安装。
