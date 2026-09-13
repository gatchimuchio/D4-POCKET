// 開発専用。秘密を表示せず、一時招待だけを認証付きdebug VM経路へ渡す。
import 'dart:convert';
import 'dart:io';

import 'package:flutter_driver/flutter_driver.dart';
import 'package:integration_test/common.dart';

Future<void> main() async {
  final invitationFile = Platform.environment['GUI_SHELL_TEST_INVITATION_FILE'];
  final root = Platform.environment['GUI_SHELL_TEST_ROOT'];
  final simulator = Platform.environment['GUI_SHELL_TEST_SIMULATOR'];
  if (invitationFile == null || root == null || simulator == null) {
    throw StateError('試験host設定が不足しています');
  }
  final platform = Platform.environment['GUI_SHELL_TEST_PLATFORM'] ?? 'ios';
  final adb = Platform.environment['GUI_SHELL_TEST_ADB'];
  Future<String> androidCommand(List<String> args) async {
    final result = await Process.run(adb!, [
      '-s',
      simulator,
      ...args,
    ]).timeout(const Duration(seconds: 10));
    if (result.exitCode != 0) throw StateError('仮想端末の操作に失敗');
    return result.stdout.toString().trim();
  }

  if (platform == 'android') {
    if (!Platform.isLinux ||
        simulator != 'emulator-5554' ||
        adb == null ||
        await androidCommand(['shell', 'getprop', 'ro.kernel.qemu']) != '1' ||
        (await androidCommand([
              'emu',
              'avd',
              'name',
            ])).split('\n').first.trim() !=
            'gui_shell_native_test') {
      throw StateError('専用Android仮想端末を確認できません');
    }
  } else if (platform != 'ios' || !Platform.isMacOS) {
    throw StateError('対応する仮想端末hostではありません');
  }
  final driver = await FlutterDriver.connect(printCommunication: false);
  if (driver is! VMServiceFlutterDriver) {
    await driver.close();
    throw StateError('認証付きVM接続を確認できません');
  }
  try {
    await driver.serviceClient.callServiceExtension(
      'ext.gui_shell_test.configure',
      isolateId: driver.appIsolate.id,
      args: {'invitation': await File(invitationFile).readAsString()},
    );
    Response? completed;
    Object? responseFailure;
    final responseFuture = driver
        .requestData(null, timeout: const Duration(minutes: 4))
        .then((raw) {
          completed = Response.fromJson(raw);
        })
        .catchError((Object error) {
          responseFailure = error;
        });
    var switched = false;
    var published = false;
    final deadline = DateTime.now().add(const Duration(minutes: 3));
    while (!published) {
      if (DateTime.now().isAfter(deadline)) throw StateError('試験準備期限を超過');
      if (completed != null || responseFailure != null) {
        throw StateError('要求準備前にSimulator試験が終了');
      }
      final status = await driver.serviceClient.callServiceExtension(
        'ext.gui_shell_test.state',
        isolateId: driver.appIsolate.id,
      );
      final phase = status.json!['phase'];
      await File('$root/simulator-state.json').writeAsString(
        jsonEncode({
          'phase': phase,
          'foreground': status.json!['foreground'],
          'ready': status.json!['ready'],
        }),
      );
      if (phase == 'background' && !switched) {
        switched = true;
        if (platform == 'android') {
          await androidCommand(['shell', 'input', 'keyevent', 'KEYCODE_HOME']);
          await Future<void>.delayed(const Duration(seconds: 2));
          await androidCommand([
            'shell',
            'am',
            'start',
            '-W',
            '-n',
            'com.example.gui_shell_mobile/.MainActivity',
          ]);
          await Future<void>.delayed(const Duration(seconds: 2));
        } else {
          for (final bundle in [
            'com.apple.Preferences',
            'com.example.guiShellMobile',
          ]) {
            final result = await Process.run('xcrun', [
              'simctl',
              'launch',
              simulator,
              bundle,
            ]);
            if (result.exitCode != 0) throw StateError('Simulatorの前景切替に失敗');
            await Future<void>.delayed(const Duration(seconds: 2));
          }
        }
      }
      if (phase == 'approval') {
        final requests = status.json!['requests'];
        if (requests is! List ||
            requests.length != 2 ||
            requests.any(
              (v) => v is! String || !RegExp(r'^[0-9a-f]{32}$').hasMatch(v),
            )) {
          throw StateError('試験要求IDが不正です');
        }
        await File(
          '$root/simulator-ready.json',
        ).writeAsString(jsonEncode({'requests': requests}));
        published = true;
      }
      await Future<void>.delayed(const Duration(milliseconds: 200));
    }
    await responseFuture;
    if (responseFailure != null || completed == null) {
      throw StateError("試験結果を取得できません");
    }
    final result = completed!;
    if (!result.allTestsPassed) throw StateError('Simulator製品経路の試験失敗');
    await File(
      '$root/simulator-result.json',
    ).writeAsString(jsonEncode({'result': 'PASS', ...?result.data}));
    print('Simulatorのnative保管・実TLS・実API・OS復帰試験がPASS');
  } finally {
    await driver.close();
  }
}
