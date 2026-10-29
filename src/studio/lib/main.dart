import 'package:flutter/material.dart';

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
      home: const _PlaceholderPage(),
    );
  }
}

/// 占位页：Studio 的界面还没有定形，先立标题与空态。
/// 首批界面按落盘契约做（待确认队列、流程看板、能力目录），
/// 读同一个 XDG 目录，不另起一套飞书调用。
class _PlaceholderPage extends StatelessWidget {
  const _PlaceholderPage();

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: const Text('量潮飞书云工作台'),
      ),
      body: const Center(
        child: Text('界面未定形，先立骨架'),
      ),
    );
  }
}
