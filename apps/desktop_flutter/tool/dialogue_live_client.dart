// 開発専用の実IPC試験。製品clientを使い、owner承認は外側の試験操作者が行う。
import 'dart:convert';
import 'dart:io';
import 'package:gui_shell_desktop/services/broker_client.dart';
import 'package:gui_shell_desktop/services/runtime_dialogue_client.dart';

Future<void> main(List<String> args) async {
  if (args.length != 2) throw ArgumentError('通常資格fileと試験出力directoryが必要です');
  final client =
      RuntimeDialogueClient(await BrokerClient.connect(sessionFile: args[0]));
  final sessions = [await client.start('left'), await client.start('right')];
  final requests = [
    await client.send(sessions[0], 'こんにちは'),
    await client.send(sessions[1], 'こんにちは')
  ];
  if (sessions[0] == sessions[1] || requests[0] == requests[1]) {
    throw StateError('要求またはsessionの混線');
  }
  final directory = Directory(args[1]);
  await File('${directory.path}/dart-ready.json')
      .writeAsString(jsonEncode({'requests': requests}));
  final deadline = DateTime.now().add(const Duration(seconds: 20));
  final results = <DialogueResult?>[null, null];
  while (results.any((v) => v == null)) {
    if (DateTime.now().isAfter(deadline)) throw StateError('実対話の待機期限');
    for (var index = 0; index < 2; index++) {
      if (results[index] != null) continue;
      final progress = await client.poll(
          requests[index], index == 0 ? 'left' : 'right', sessions[index]);
      if (progress.record == null) throw StateError('実行記録が返っていない');
      if (progress.result != null &&
          (progress.record!.fields['開始時刻'] == null ||
              progress.record!.fields['終了時刻'] == null)) {
        throw StateError('実対話の開始・終了記録が確定していない');
      }
      results[index] = progress.result;
    }
    await Future<void>.delayed(const Duration(milliseconds: 100));
  }
  if (results.any((v) =>
      v!.text('状態') != '成功' ||
      v.text('本文').isEmpty ||
      v.text('追跡hash').isEmpty)) {
    throw StateError('実対話の本文または追跡が成立しない');
  }
  for (final session in sessions) {
    await client.close(session);
  }
  await File('${directory.path}/dart-result.json').writeAsString(
      jsonEncode({'result': 'PASS', 'evidence_source': 'LIVE_RUNTIME'}));
}
