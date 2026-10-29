# qtcloud-feishu Studio

Flutter 工作台，与 CLI 共用同一套文件契约（资源登记、流程定义、落盘与游标）。

CLI 是执行与落盘的一端，Studio 是看与确认的一端。Studio 不实现第二套飞书调用：需要「查」的能力时，走 CLI 的库目标封装，而不是直接调 lark-cli。

## 现状

起点页是**待确认队列**（`lib/screens/draft_queue.dart`）：CLI 落盘的草稿在这里等人确认，确认后由执行器真正写回，页面不自己发信。

选它做起点是因为它是这套架构里唯一需要人参与才成立的界面：CLI 单独就能跑流程，但「发信默认草稿、人确认后才发送」这条原则要有这一页才闭环；且它消费的契约最窄——只有落盘里的草稿记录。

- 数据源未接：草稿动作在 CLI 迁入清单第 5 步（邮件发送通道），落盘记录出现后从落盘目录读，字段以落盘为准
- 确认/驳回按钮先禁用：写回方式随落盘契约一起定，接上后再放开
- 后续界面：流程看板（推格状态）、能力目录（从登记生成表单）

## 本地开发

```bash
cd src/studio
flutter pub get
flutter run -d chrome
```

## 数据源

待确认队列直读 CLI 的本机落盘（桌面形态，`lib/services/draft_source.dart`）：

- 待确认条目：`<state>/qtcloud-feishu/<流程名>.json` 里 `confirmed` 为空且 `artifacts.草稿id` 非空
- 草稿内容：`raw/mail/draft-create/` 信封优先，`to`/`subject` 缺时从 `ops.jsonl` 兑底，字段缺失原样报出
- 读不到时区分「数据源没接上」与「队列为空」

web 静态产物读不了本地目录——这条通路只在桌面形态成立，上云时把 `DraftSource` 换成服务封装，形状不变。确认写回（按钮启用）是 ROADMAP 阶段三，现在按钮禁用。

实施顺序与设计反思见 [ROADMAP.md](ROADMAP.md)。

## 验证

```bash
flutter analyze
flutter test
flutter build web --release --base-href /
```
