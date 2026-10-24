## Purpose

规定笔记的阅读与编辑体验：默认只读预览、按需进入源码编辑，围栏代码块可逐块复制，渲染过程不得执行笔记中的 HTML 或脚本。

## ADDED Requirements

### Requirement: 阅读与编辑双模式

打开笔记 MUST 默认进入阅读模式；用户 MUST 能通过界面控件或 `Command+E` 进入编辑模式，并通过完成按钮或 `Esc` 返回阅读模式。

#### Scenario: 打开笔记为阅读态
- **WHEN** 用户点击列表中的笔记
- **THEN** 右侧以渲染后的 Markdown 呈现正文，不出现编辑器光标

#### Scenario: 快捷键切换模式
- **WHEN** 阅读状态下按 `Command+E`
- **THEN** 正文切换为源码编辑状态且光标可用；再按一次返回阅读状态

#### Scenario: 退出编辑先保存
- **WHEN** 编辑中有未落盘内容时按 `Esc`
- **THEN** 系统先完成保存再切回阅读模式，不丢弃输入

### Requirement: 源码编辑器

编辑模式 MUST 使用 CodeMirror 6 的 Markdown 编辑器，MUST 提供与预览一致的字号设置与主题，且 MUST NOT 引入第二套编辑器实现。

#### Scenario: 编辑器继承外观设置
- **WHEN** 用户把编辑器字号从 15px 调到 18px
- **THEN** 重新进入编辑模式时代码字号为 18px

### Requirement: 渲染安全性

预览 MUST 禁用原始 HTML，MUST 对链接做协议白名单校验，MUST NOT 执行 `javascript:` 等可执行协议。

#### Scenario: 内联脚本不执行
- **WHEN** 笔记正文包含 `<script>alert(1)</script>`
- **THEN** 预览将其作为纯文本显示，不产生脚本执行

#### Scenario: 危险链接被拦截
- **WHEN** 正文包含 `[x](javascript:alert(1))`
- **THEN** 链接不触发脚本执行，且不进行外部跳转

#### Scenario: 伪协议变体同样被拦截
- **WHEN** 正文包含大小写混写、夹带控制字符（如 `java\tscript:`）的伪协议链接，或协议相对链接（`//host/path`）
- **THEN** 预览不生成可点击的链接，也不继承页面协议发起跳转

#### Scenario: 非 http(s) 链接不使应用离开界面
- **WHEN** 用户点击正文中的 `mailto:`、相对路径或锚点链接
- **THEN** 应用内不导航、不打开新的网页视图，界面保持在当前笔记

### Requirement: 围栏代码块复制

每个围栏代码块 MUST 独立渲染并带复制按钮；复制内容 MUST 等于代码正文（去掉围栏标记与语言标识），MUST 保留原有换行与缩进。

#### Scenario: 复制单个代码块
- **WHEN** 用户点击某个代码块的复制按钮
- **THEN** 剪贴板内容为该代码块原文，不含 ```` ```lang ```` 行与结尾围栏

#### Scenario: 多代码块互不串扰
- **WHEN** 笔记含 Bash、TypeScript 两个代码块
- **THEN** 分别复制各自内容，互不包含另一块内容

#### Scenario: 未闭合围栏仍可读
- **WHEN** 笔记以未闭合的代码围栏结束
- **THEN** 预览不报错，剩余内容作为代码块呈现并可复制

### Requirement: 全文复制

阅读模式 MUST 提供复制整篇 Markdown 的操作，复制内容 MUST 为笔记原始 Markdown 源码，而非渲染文本。

#### Scenario: 复制全文保留 Markdown
- **WHEN** 用户点击标题区的复制全文按钮
- **THEN** 剪贴板内容为包含 `#` 标题与围栏标记的原始 Markdown

### Requirement: 语法高亮

预览中的围栏代码块 MUST 按语言标注进行高亮；无法识别或未标注语言时 MUST 以纯文本等宽呈现，且 MUST NOT 因高亮失败导致整篇预览失败。语法定义 MUST 按需加载（不进入首屏包），单个代码块超过 2 万字符时 MUST 降级为纯文本，高亮结果 MUST 随深浅色主题切换，且代码块文字与其背景的对比度 MUST 不低于 4.5:1。

#### Scenario: 常用语言高亮
- **WHEN** 笔记含 ```bash、```typescript、```sql、```json 代码块
- **THEN** 四类代码块均按对应语法着色

#### Scenario: 高亮失败降级
- **WHEN** 代码块标注了不存在的语言
- **THEN** 该块以纯文本显示，其余内容正常渲染

#### Scenario: 高亮跟随主题
- **WHEN** 笔记含代码块，用户把外观从浅色切到深色
- **THEN** 代码块改用深色配色重新着色，不需要重新打开笔记
