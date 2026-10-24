# vivin 仓库开发约定

本文件约束在 **vivin 源码仓库** 中工作的编码代理。应用以后在用户项目文件夹中创建的 `AGENTS.md` 是该项目的写作规则，两者用途不同，不得互相复制或混用。

## 依据和范围

- 产品行为以 `docs/requirements.md` 为准；界面细节以 `docs/design/macos-ui-spec.md` 为准，概念图提供视觉方向。
- 这是单应用仓库，不照搬参考项目的 monorepo、后台 API、权限或发布流程。
- 改动保持在满足当前任务所需的最小范围，保留用户未提交的改动。新增依赖时说明用途，并更新锁文件。

## 技术栈与职责

- 前端：Vue 3、TypeScript、Vite、Tailwind CSS 4；使用 pnpm。
- 桌面与本地能力：Tauri 2 / Rust。项目文件读写、路径边界、锁定校验、修改记录和本机 AI 接口属于 Rust 侧或明确的共享领域服务，不让 Vue 组件直接操作任意磁盘路径。
- 排版 Markdown 编辑：Milkdown Crepe；原文编辑：CodeMirror 6。两种视图的持久化格式都是普通 `.md`，不得把术语、标注或应用状态写进正文。增加 Markdown 语法支持时，先验证往返保存不会丢失用户原文。
- Tauri 命令使用明确的请求和返回类型；外部输入按 `unknown` 处理并校验。所有路径先限定在已打开项目根目录内，检查规范化后的实际路径及符号链接，拒绝越界访问。每个写命令验证项目、锁定状态和文档版本，返回可解释的错误。
- Tauri capability 只授予当前功能所需权限。禁止以全盘读写权限代替具体的项目边界检查。

## 目录和命名

- `src/` 放 Vue 界面；功能逐步归入 `src/features/<feature>/`，跨功能复用组件放 `src/components/`，无界面依赖的纯逻辑放 `src/lib/`。不要为尚不存在的功能创建空目录。
- `src-tauri/src/` 放原生入口、命令和领域服务。Rust 服务不依赖前端组件；前端通过单独的 Tauri 适配层调用命令，不在多个组件中散写 `invoke` 字符串。
- 文件与目录使用 `kebab-case`；Vue 组件和 TypeScript 类型使用 `PascalCase`；变量与函数使用 `camelCase`；composable 以 `use` 开头。Rust 按社区命名惯例。
- 页面组件负责编排；共享响应式逻辑放 composable，纯转换放纯函数。跨栏位共享的文档内容只维护一份状态，栏位自身只持有选区、滚动和视图模式等窗口状态。

## 质量和验证

- 遵循 `.editorconfig`：UTF-8、LF、2 空格；Rust 用 4 空格。TypeScript 使用严格类型，避免无理由的 `any`、非空断言和多层强转。
- 使用 Oxlint 与 Oxfmt，不能再并行引入另一套默认 lint/format 配置。常用命令：`pnpm typecheck`、`pnpm lint`、`pnpm format:check`、`pnpm build`；Rust 可用时执行 `cargo fmt --check`、`cargo check`。
- 测试聚焦领域规则：项目隔离、锁定继承、外部修改冲突、双栏同步、Markdown 往返、标注定位与 AI 撤销。根据改动风险选择最小有效验证；不写只复述实现的测试。
- 修改后说明变更文件、原因、运行过的检查和未验证的风险。提交信息使用 `type(scope): summary`，如 `feat(editor): add source mode`；不在没有要求时自动提交。
