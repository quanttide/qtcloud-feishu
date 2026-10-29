# qtcloud-feishu Studio

Flutter 工作台，与 CLI 共用同一套文件契约（资源登记、流程定义、落盘与游标）。

CLI 是执行与落盘的一端，Studio 是看与确认的一端。Studio 不实现第二套飞书调用：需要「查」的能力时，走 CLI 的库目标封装，而不是直接调 lark-cli。

## 现状

命令面和界面都未定形，当前是骨架：`lib/main.dart` 只有标题与空态占位页。首批界面按落盘契约做：

- 待确认队列：CLI 落盘的草稿（如发信默认草稿），人确认后执行器才真正写回
- 流程看板：流程 YAML 的推格状态，一张卡 = 一个格子
- 能力目录：从资源登记生成的参数表单

## 本地开发

```bash
cd src/studio
flutter pub get
flutter run -d chrome
```

Studio 读 CLI 的 XDG 落盘目录，两边同库同状态；要多人协作再考虑共享存储或服务化，界面与契约不变。

## 验证

```bash
flutter analyze
flutter test
flutter build web --release --base-href /
```
