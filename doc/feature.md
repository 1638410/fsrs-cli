# fsrs-cli 特性说明

## 这是什么？

`fsrs-cli` 是一个运行在终端里的记忆管理工具。它帮你把 Markdown 笔记变成一张张"知识卡片"，然后用间隔重复算法（FSRS）安排你什么时候该复习什么。

简单说：**它是你的第二大脑，住在命令行里。**

---

## 开发环境

- **语言**：Rust（edition 2021）
- **工具链**：通过 `toolbox enter box` 进入开发容器后使用 cargo/rustc
- **构建**：`cargo build --release`
- **测试**：`cargo test`

```bash
# 进入开发环境
toolbox enter box

# 构建项目
cd fsrs-cli
cargo build --release

# 带 AI 功能构建
cargo build --release --features ai

# 带 PDF 功能构建
cargo build --release --features pdf
```

---

## 核心功能

### 1. 导入知识

把写好的 Markdown 文件丢给它，它会自动识别里面的问答卡片并导入。

```bash
# 导入单个文件
fsrs-cli import notes/rust-borrowing.md

# 导入整个目录
fsrs-cli import ./notes/

# 从 tab 分隔的 CSV 导入（无表头，\n 换行，| 标签）
fsrs-cli csv-import cards.csv
```

Markdown / CSV 文件保持原位不动，工具只是读取并索引内容。

### 2. 交互式制卡

不想手写 Markdown？用 TUI 内部 editor 制作卡组，参考 glow 风格：

```bash
# 必填输出文件（追加到现有 .md 或新建）
fsrs-cli make notes/daily.md
```

- 多行答案（`:end` 或 Ctrl+D 结束）
- 答案支持 Markdown（bold/italic/header/list/code）
- 标签 `|` 分隔
- 退出后自动 import 到记忆库
- 退出未保存直接丢弃

### 3. 间隔复习

这是最核心的功能。工具会根据 FSRS 算法计算每张卡片的"到期日"，到期的卡片会出现在复习队列里。

```bash
# 开始复习
fsrs-cli review

# 只复习 10 张
fsrs-cli review --limit 10
```

复习过程：
- 看到问题 → 想一想 → 按空格看答案 → 给自己打分（忘了/困难/一般/简单）
- 每打完一张分，进度就保存一次。随时 Ctrl+C 退出，下次接着来。

### 4. 全文搜索

忘了某张卡片？用关键词搜。

```bash
fsrs-cli search "所有权"
fsrs-cli search "lifetime" --limit 5
```

搜索结果会标注每张卡片的状态：是否到期、还剩几天。

### 5. 更新内容

改了笔记文件？重新导入，工具会自动检测哪些卡片变了。

```bash
# 更新单个文件（问题变了会重置复习进度）
fsrs-cli update notes/rust-borrowing.md

# 保留复习进度（即使问题变了）
fsrs-cli update notes/rust-borrowing.md --keep-schedule
```

### 6. 学习统计

看看自己的学习情况。

```bash
fsrs-cli stats
fsrs-cli stats --period 30  # 最近 30 天
```

### 7. 参数优化

积累了一定复习数据后，可以优化 FSRS 算法参数，让它更准确地预测你的记忆曲线。

```bash
fsrs-cli optimize
```

---

## AI 辅助功能

AI 功能是可选的，需要编译时启用 `--features ai`。

### AI 生成卡片

给它一篇文章或一个主题，AI 帮你自动生成问答卡片。生成的卡片先进"草稿箱"，你确认后才正式入库。

```bash
fsrs-cli ai generate article.md
fsrs-cli ai generate "Rust 所有权规则"
```

### AI 自动标签

让 AI 给你的卡片打标签，方便分类。

```bash
fsrs-cli ai tag <card-id>
fsrs-cli ai tag --batch  # 批量处理所有未标记的卡片
```

### AI 健康检查

扫描整个知识库，告诉你哪些卡片有问题：答案太短、多张卡片内容重复、太久没复习等。

```bash
fsrs-cli ai health
```

### AI 扩充答案

一句话的答案太简略？让 AI 帮你补充详细解释。

```bash
fsrs-cli ai expand <card-id>
```

### AI 复习建议

根据 AI 访问卡片的频率，推荐你优先复习哪些卡片。

```bash
fsrs-cli ai review-suggest
```

---

## 怎么写卡片？

在 Markdown 文件里用 `<!-- card -->` 标记包裹问答对：

```markdown
---
tags: [rust, ownership]
---

这是一些普通的笔记内容，不会被识别为卡片。

<!-- card -->
Q:: Rust 中一条值同时只能有几个可变引用？
A:: 恰好一个。
<!-- card -->

<!-- card -->
Q:: 什么是借用？
A:: 借用是访问值而不获取其所有权的方式。用 & 表示不可变借用，&mut 表示可变借用。
<!-- card }}
```

规则很简单：
- `<!-- card -->` 开始，`<!-- card -->` 结束
- `Q::` 后面是问题
- `A::` 后面是答案
- 中间的普通笔记会被忽略

---

## 存储结构

工具在项目目录下创建 `.fsrs/` 文件夹：

```
your-project/
├── .fsrs/
│   ├── memory.db       # 数据库（卡片、复习记录等）
│   └── tantivy/        # 搜索索引
├── notes/
│   ├── rust-ownership.md   # 你的笔记（原位不动）
│   └── rust-lifetime.md
└── ...
```

所有数据都在本地，不依赖任何云服务。

---

## 为什么用 FSRS？

传统的间隔重复算法（如 Anki 默认的 SM-2）是上世纪 80 年代的设计。FSRS 是基于现代机器学习的算法，它能更准确地预测你的记忆衰减曲线，用更少的复习次数达到同样的记忆效果。

简单说：**同样的知识，用 FSRS 复习次数更少，但记得更牢。**

---

## 安全设计

- **数据主权在你**：所有文件和数据都在本地，不上传任何东西
- **复习记录纯净**：AI 不会污染你的复习数据，它们是完全隔离的
- **变更安全**：修改卡片内容后，默认重置复习进度（防止用错误的预测安排复习）
- **随时中断**：复习过程中随时退出，已评分的卡片会保存，当前正在复习的不会丢失

---

## 依赖

核心功能不需要任何外部服务。AI 功能需要 OpenAI API Key（或兼容的 API）。

---

## 项目状态

- [ ] 项目初始化
- [ ] 核心数据结构
- [ ] SQLite 存储层
- [ ] Markdown 解析器
- [ ] FSRS 调度器集成
- [ ] import 命令
- [ ] review 命令（TUI）
- [ ] search 命令（Tantivy）
- [ ] update 命令
- [ ] stats 命令
- [ ] optimize 命令
- [ ] AI 辅助模块（可选）
- [ ] 测试
- [ ] 文档完善
