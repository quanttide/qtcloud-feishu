import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:qtcloud_feishu_studio/main.dart';
import 'package:qtcloud_feishu_studio/screens/draft_queue.dart';

void main() {
  testWidgets('起点页是待确认队列，空态显示没有草稿', (WidgetTester tester) async {
    await tester.pumpWidget(const StudioApp());

    expect(find.text('待确认'), findsOneWidget);
    expect(find.text('没有待确认的草稿'), findsOneWidget);
  });

  testWidgets('有草稿时显示卡片与两个未启用的按钮', (WidgetTester tester) async {
    final draft = Draft(
      action: 'mail_send',
      recipient: 'candidate@example.com',
      body: '邀请加入飞书群，链接见正文',
      recordedAt: DateTime(2026, 10, 29),
    );
    await tester.pumpWidget(MaterialApp(home: DraftQueueScreen(drafts: [draft])));

    expect(find.text('mail_send · candidate@example.com'), findsOneWidget);
    expect(find.text('邀请加入飞书群，链接见正文'), findsOneWidget);
    // 写回方式未定，按钮先禁用。
    final confirm = tester.widget<FilledButton>(find.widgetWithText(FilledButton, '确认发送'));
    final reject = tester.widget<OutlinedButton>(find.widgetWithText(OutlinedButton, '驳回'));
    expect(confirm.onPressed, isNull);
    expect(reject.onPressed, isNull);
  });
}
