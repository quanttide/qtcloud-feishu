import 'package:flutter/material.dart';

import 'screens/draft_queue.dart';
import 'services/draft_source.dart';

void main() {
  runApp(const StudioApp());
}

class StudioApp extends StatelessWidget {
  const StudioApp({super.key, this.source});

  /// 数据源缺省直读本机 XDG 目录；测试注入假的。
  final DraftSource? source;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: '量潮飞书云工作台',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
      ),
      // 起点页：待确认队列。流程看板、能力目录往后加导航再上。
      home: DraftQueueScreen(source: source),
    );
  }
}
