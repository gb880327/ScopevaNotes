# release-readiness Specification

## Purpose
定义可交付的质量基线：常驻资源占用与响应延迟的实测上限、数据安全与恢复演练、打包与签名分发要求，以及必须随版本更新的文档（CHANGELOG、RELEASE、README）。

## Requirements

### Requirement: 常驻资源占用

隐藏到菜单栏后，应用 MUST 在 24 小时驻留中保持稳定：空闲 CPU MUST 接近 0，内存 MUST NOT 随驻留时间持续增长。

#### Scenario: 空闲驻留
- **WHEN** 应用隐藏到菜单栏并空闲
- **THEN** 进程无持续 CPU 占用，内存曲线无单调增长（v1 以 `scripts/perf-check.sh soak` 采样，采样时长由发布者决定，见 D20）

### Requirement: 唤出与启动延迟

从触发全局快捷键到窗口可见 MUST 在 200ms 内开始呈现；冷启动到可交互 MUST 在 3s 内完成（10,000 条笔记规模）。

#### Scenario: 热唤出延迟
- **WHEN** 应用已驻留且用户按下全局快捷键
- **THEN** 窗口在 200ms 内可见并可接受键盘输入（该指标无法自动化，由发布者按键实测，见 D20）

### Requirement: 数据安全演练

发布前 MUST 完成并通过：迁移失败回滚、写入过程中强制结束进程后重启、备份文件还原、只读目录打开失败、schema 版本过新拒绝写入。

#### Scenario: 强制结束后恢复
- **WHEN** 保存过程中进程被强制结束，随后重新启动
- **THEN** 数据库可正常打开，最后一次已提交的编辑完整，未提交编辑不产生半写记录

#### Scenario: 备份还原
- **WHEN** 用备份文件替换当前笔记库并启动
- **THEN** 笔记、标签与搜索均可正常工作

### Requirement: 打包与分发

构建 MUST 产出可运行的 `.app`。v1 的交付形态是**本机自用**：产物使用 ad-hoc 签名（`codesign --sign -`），不做 Developer ID 签名与 notarization——这不是遗漏，是范围决定（见 design D20）。若将来对外分发，该变更 MUST 补齐签名、公证与 Gatekeeper 校验，并在此之前不得声称产物可对外分发。

#### Scenario: 本地构建可用
- **WHEN** 执行发布构建
- **THEN** 生成的 `.app` 可在本机启动并读写笔记，`lipo -info` 显示目标架构

#### Scenario: 本机自用不做签名公证
- **WHEN** 用户把构建产物放到另一台机器或经网络传输
- **THEN** 发布文档明确说明当前只有 ad-hoc 签名、需要 `xattr -dr com.apple.quarantine` 或右键打开，且这不是受支持的对外分发方式

### Requirement: 发布文档

每个发布 MUST 附带：安装与首次使用说明、快捷键说明、笔记库位置与备份方法、已知限制。

#### Scenario: 用户能找到数据位置
- **WHEN** 用户阅读发布文档
- **THEN** 文档明确写出默认笔记库路径与备份/迁移步骤
