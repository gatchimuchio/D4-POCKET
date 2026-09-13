// 開発専用。実TLS・native保管・製品画面のOS lifecycleを接続する。
import 'dart:async';
import 'dart:convert';
import 'dart:developer';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_mobile/main.dart';
import 'package:gui_shell_mobile/services/device_link_client.dart';
import 'package:gui_shell_mobile/services/device_link_controller.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';
import 'package:integration_test/integration_test.dart';

import 'native_store.dart';

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  final configuration = Completer<String>();
  final state = <String, Object?>{'phase': 'configuration'};
  registerExtension('ext.gui_shell_test.configure', (_, parameters) async {
    final raw = parameters['invitation'];
    if (configuration.isCompleted || raw == null || raw.length > 8192) {
      return ServiceExtensionResponse.error(-32000, '試験設定を受理できません');
    }
    configuration.complete(raw);
    return ServiceExtensionResponse.result('{"received":true}');
  });
  registerExtension('ext.gui_shell_test.state', (_, parameters) async {
    return ServiceExtensionResponse.result(jsonEncode(state));
  });

  testWidgets(
    'Simulatorでnative保管・再接続・実API対話・OS復帰・失効を検証',
    (tester) async {
      expect(Platform.isIOS, isTrue, reason: 'このdriverのOS遷移操作はiOS専用');
      final raw = await configuration.future.timeout(
        const Duration(seconds: 60),
      );
      final device = strictObject(raw)['端末ID'] as String;
      final store = ScopedNativeStore('gui_shell_live_${newDeviceId()}_');
      addTearDown(store.clear);
      await store.write(DeviceLinkController.deviceKey, device);
      final first = DeviceLinkController(store: store);
      var firstDisposed = false;
      addTearDown(() {
        if (!firstDisposed) first.dispose();
      });
      await first.initialize();
      expect(first.storageReady, isTrue);
      final invite = first.invitation(raw);
      final incorrect = DeviceCredential.parse(
        jsonEncode({...invite.data, '証明書hash': '0' * 64}),
        invitation: true,
        deviceId: device,
      );
      await expectLater(
        DeviceLinkClient(incorrect).pair(),
        throwsA(isA<BrokerClientException>()),
      );
      await first.pair(invite);
      expect(first.ready, isTrue, reason: first.status);
      expect(first.runtimes, ['left', 'right']);
      first.dispose();
      firstDisposed = true;

      // メモリ資格を引継がず、native再読取と実TLS照合から接続し直す。
      final controller = DeviceLinkController(store: store);
      addTearDown(controller.dispose);
      await controller.initialize();
      expect(controller.ready, isTrue, reason: controller.status);
      await tester.pumpWidget(GuiShellMobileApp(controller: controller));
      await tester.pump();
      var backgroundObserved = false;
      var resumeObserved = false;
      var stoppedInBackground = false;
      void observe() {
        state.addAll({
          "foreground": controller.foreground,
          "ready": controller.ready,
        });
        if (!controller.foreground) {
          backgroundObserved = true;
          stoppedInBackground = !controller.ready;
        } else if (backgroundObserved) {
          resumeObserved = true;
        }
      }

      controller.addListener(observe);
      addTearDown(() => controller.removeListener(observe));
      state['phase'] = 'background';
      final lifecycleDeadline = DateTime.now().add(const Duration(seconds: 40));
      while (!resumeObserved || !controller.ready) {
        if (DateTime.now().isAfter(lifecycleDeadline)) {
          fail('OS背景遷移と復帰の確認期限を超過');
        }
        await Future<void>.delayed(const Duration(milliseconds: 100));
      }
      expect(backgroundObserved && stoppedInBackground, isTrue);
      expect(controller.runtimes, ['left', 'right']);

      final client = controller.dialogue;
      final sessions = [
        await client.start('left'),
        await client.start('right'),
      ];
      final requests = [
        await client.send(sessions[0], 'こんにちは'),
        await client.send(sessions[1], 'こんにちは'),
      ];
      expect(sessions[0] == sessions[1] || requests[0] == requests[1], isFalse);
      state.addAll({'phase': 'approval', 'requests': requests});
      final results = <DialogueResult?>[null, null];
      final deadline = DateTime.now().add(const Duration(seconds: 30));
      while (results.any((v) => v == null)) {
        if (DateTime.now().isAfter(deadline)) fail('実API応答の確認期限を超過');
        for (var i = 0; i < 2; i++) {
          results[i] ??= (await client.poll(
            requests[i],
            i == 0 ? 'left' : 'right',
            sessions[i],
          )).result;
        }
        await Future<void>.delayed(const Duration(milliseconds: 100));
      }
      expect(
        results.every(
          (v) =>
              v!.text('状態') == '成功' &&
              v.text('本文').isNotEmpty &&
              v.text('追跡hash').isNotEmpty,
        ),
        isTrue,
      );
      for (final session in sessions) {
        await client.close(session);
      }

      final stored = await store.read(DeviceLinkController.credentialKey);
      expect(stored != null, isTrue);
      await store.delete(DeviceLinkController.credentialKey);
      await controller.verify();
      expect(controller.ready, isFalse);
      await expectLater(
        controller.request('実行系列挙'),
        throwsA(isA<BrokerClientException>()),
      );
      await store.write(DeviceLinkController.credentialKey, stored!);
      await controller.verify();
      expect(controller.ready, isTrue);
      final revokedClient = DeviceLinkClient(controller.credential!);
      await controller.disconnect();
      expect(controller.hasStoredCredential, isFalse);
      expect(await store.read(DeviceLinkController.credentialKey), isNull);
      await expectLater(
        revokedClient.request('端末確認'),
        throwsA(isA<BrokerClientException>()),
      );
      binding.reportData = {
        'evidence_source': 'LIVE_RUNTIME',
        'native_storage': true,
        'controller_recreated': true,
        'os_background_stopped': stoppedInBackground,
        'os_resume_reconnected': resumeObserved,
        'real_runtime_results': 2,
        'revoked_credential_rejected': true,
        'physical_device_verified': false,
      };
      state['phase'] = 'complete';
    },
    timeout: const Timeout(Duration(minutes: 4)),
  );
}
