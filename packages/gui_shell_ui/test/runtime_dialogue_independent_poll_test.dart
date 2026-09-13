import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';
import 'runtime_dialogue_test.dart' as fixture;

class DelayedPoll extends fixture.DialogueFixture {
  DelayedPoll(this.runtime);
  final String runtime;
  final leftWait = Completer<void>();
  int waitingCalls = 0;
  @override
  Future<Map<String, Object?>> request(String op,
      {Map<String, Object?>? payload}) async {
    if (op == '対話取得' && sessions[requests[payload!['要求ID']]] == runtime) {
      waitingCalls++;
      await leftWait.future;
    }
    return super.request(op, payload: payload);
  }
}

void main() {
  for (final runtime in ['left', 'right']) {
    testWidgets('$runtimeの照会待機中も他側の完了を表示する', (tester) async {
      tester.view.physicalSize = const Size(1400, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final f = DelayedPoll(runtime)..complete = true;
      await tester.pumpWidget(MaterialApp(
          home: Scaffold(
              body: RuntimeDialogueScreen(
                  connect: () async => RuntimeDialogueClient(f)))));
      await tester.pumpAndSettle();
      await tester.tap(find.byType(Switch));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), '同一入力');
      await tester.tap(find.byKey(const ValueKey('dialogue-send')));
      await tester.pumpAndSettle();
      await tester.pump(const Duration(seconds: 1));
      await tester.pumpAndSettle();
      final rightVisible = find
          .text('${runtime == 'left' ? 'right' : 'left'}の応答')
          .evaluate()
          .length;
      await tester.pump(const Duration(seconds: 3));
      expect(f.waitingCalls, 1);
      f.leftWait.complete();
      await tester.pumpAndSettle();
      expect(find.text('leftの応答'), findsOneWidget);
      expect(find.text('rightの応答'), findsOneWidget);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      expect(rightVisible, 1);
    });
  }
}
