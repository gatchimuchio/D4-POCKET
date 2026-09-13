import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_mobile/services/device_link_controller.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';
import 'package:integration_test/integration_test.dart';

import 'native_store.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() {
    expect(
      Platform.isIOS || Platform.isAndroid,
      isTrue,
      reason: 'Mobile native環境以外を安全保管の実行証拠にしない',
    );
  });

  testWidgets('native安全保管の書込・別instance読取・更新・削除', (tester) async {
    final prefix =
        'gui_shell_integration_${DateTime.now().microsecondsSinceEpoch}_';
    final store = ScopedNativeStore(prefix);
    addTearDown(store.clear);
    expect(await store.read('probe'), isNull);
    await store.write('probe', '試験専用の非秘密値');
    final reopened = ScopedNativeStore(prefix);
    expect(await reopened.read('probe'), '試験専用の非秘密値');
    await store.write('probe', '更新した試験値');
    expect(await reopened.read('probe'), '更新した試験値');
    await store.delete('probe');
    expect(await reopened.read('probe'), isNull);
  });

  testWidgets('native保管の端末ID再読取と破損資格の通信停止', (tester) async {
    final store = ScopedNativeStore(
      'gui_shell_integration_${DateTime.now().microsecondsSinceEpoch}_',
    );
    addTearDown(store.clear);
    final first = DeviceLinkController(store: store);
    await first.initialize();
    expect(first.storageReady, isTrue);
    expect(first.ready, isFalse);
    final deviceId = first.deviceId;
    expect(deviceId, matches(RegExp(r'^[0-9a-f]{32}$')));
    first.dispose();

    // 実native保管を再読取し、資格なしを接続済みにしない。
    final reopened = DeviceLinkController(store: store);
    addTearDown(reopened.dispose);
    await reopened.initialize();
    expect(reopened.deviceId, deviceId);
    expect(reopened.ready, isFalse);
    reopened.setForeground(false);
    await expectLater(
      reopened.request('実行系列挙'),
      throwsA(isA<BrokerClientException>()),
    );
    reopened.setForeground(true);
    await reopened.verify();
    expect(reopened.ready, isFalse);

    await store.write(DeviceLinkController.credentialKey, '{}');
    var connections = 0;
    final corrupt = DeviceLinkController(
      store: store,
      connect: (_) {
        connections++;
        throw StateError('破損資格から接続してはならない');
      },
    );
    addTearDown(corrupt.dispose);
    await corrupt.initialize();
    expect(corrupt.ready, isFalse);
    expect(connections, 0);
    await expectLater(
      corrupt.request('実行系列挙'),
      throwsA(isA<BrokerClientException>()),
    );
    await corrupt.disconnect(localOnly: true);
    expect(await store.read(DeviceLinkController.credentialKey), isNull);
    expect(corrupt.hasStoredCredential, isFalse);
  });
}
