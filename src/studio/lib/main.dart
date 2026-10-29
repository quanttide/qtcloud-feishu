import 'package:flutter/material.dart';

import 'screens/draft_queue.dart';

void main() {
  runApp(const StudioApp());
}

class StudioApp extends StatelessWidget {
  const StudioApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: '量潮飞书云工作台',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
      ),
      // 起点页：待确认队列。流程看板、能力目录往后加导航再上。
      home: const DraftQueueScreen(),
    );
  }
}
