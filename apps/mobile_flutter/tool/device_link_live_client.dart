// 開発専用driver。製品のTLS clientと共有対話clientを実行し、owner資格は読まない。
import 'dart:convert';
import 'dart:io';
import 'package:gui_shell_mobile/services/device_link_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';

Future<void> main(List<String> args) async {
  if (args.length != 2) throw ArgumentError('試験招待fileと出力directoryが必要です');
  final raw = await File(args[0]).readAsString();
  final device = strictObject(raw)['端末ID'] as String;
  final invite = DeviceCredential.parse(
    raw,
    invitation: true,
    deviceId: device,
  );
  final incorrect = DeviceCredential.parse(
    jsonEncode({...invite.data, '証明書hash': '0' * 64}),
    invitation: true,
    deviceId: device,
  );
  var rejected = false;
  try {
    await DeviceLinkClient(incorrect).pair();
  } on BrokerClientException {
    rejected = true;
  }
  if (!rejected) throw StateError('異なる証明書を受理した');
  final credential = await DeviceLinkClient(invite).pair();
  final link = DeviceLinkClient(credential);
  await link.request('端末確認');
  link.setActive(false);
  rejected = false;
  try {
    await link.request('端末確認');
  } on BrokerClientException {
    rejected = true;
  }
  if (!rejected) throw StateError('停止中に通信した');
  link.setActive(true);
  await link.request('端末確認');
  final client = RuntimeDialogueClient(link);
  final sessions = [await client.start('left'), await client.start('right')];
  final requests = [
    await client.send(sessions[0], 'こんにちは'),
    await client.send(sessions[1], 'こんにちは'),
  ];
  if (sessions[0] == sessions[1] || requests[0] == requests[1])
    throw StateError('対話の混線');
  await File(
    '${args[1]}/mobile-dart-ready.json',
  ).writeAsString(jsonEncode({'requests': requests}));
  final deadline = DateTime.now().add(const Duration(seconds: 20));
  final results = <DialogueResult?>[null, null];
  while (results.any((v) => v == null)) {
    if (DateTime.now().isAfter(deadline)) throw StateError('対話期限');
    for (var i = 0; i < 2; i++) {
      if (results[i] == null)
        results[i] = (await client.poll(
          requests[i],
          i == 0 ? 'left' : 'right',
          sessions[i],
        )).result;
    }
    await Future<void>.delayed(const Duration(milliseconds: 100));
  }
  if (results.any(
    (v) =>
        v!.text('状態') != '成功' ||
        v.text('本文').isEmpty ||
        v.text('追跡hash').isEmpty,
  )) {
    throw StateError('本文・追跡が成立しない');
  }
  for (final session in sessions) {
    await client.close(session);
  }
  await link.request('端末離脱');
  rejected = false;
  try {
    await link.request('端末確認');
  } on BrokerClientException {
    rejected = true;
  }
  if (!rejected) throw StateError('失効済み資格を受理した');
  await File('${args[1]}/mobile-dart-result.json').writeAsString(
    jsonEncode({'result': 'PASS', 'evidence_source': 'LIVE_RUNTIME'}),
  );
}
