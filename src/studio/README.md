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

Studio 读 CLI 的 XDG 落盘目录，两边同库同状态；要多人协作再考虑共享存储或服务化，界面与契约不变。注意：这条通路只在桌面形态成立，web 静态产物读不了本地目录——形态选择见 [../ROADMAP.md](../ROADMAP.md) 阶段二。

实施顺序与设计反思见 [ROADMAP.md](ROADMAP.md)。

## 验证

```bash
flutter analyze
flutter test
flutter build web --release --base-href /
```
