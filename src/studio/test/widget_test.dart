import 'package:flutter_test/flutter_test.dart';

import 'package:qtcloud_feishu_studio/main.dart';

void main() {
  testWidgets('骨架占位页显示标题与空态', (WidgetTester tester) async {
    await tester.pumpWidget(const StudioApp());

    expect(find.text('量潮飞书云工作台'), findsOneWidget);
    expect(find.text('界面未定形，先立骨架'), findsOneWidget);
  });
}
