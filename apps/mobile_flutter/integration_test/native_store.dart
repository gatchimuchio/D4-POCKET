import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_mobile/services/device_link_controller.dart';

/// 試験キーだけを分離する。読書きは製品のnative保管APIを使用する。
/// メモリ保管やMethodChannelの置換を行わず、製品資格キーに触れない。
class ScopedNativeStore implements DeviceStore {
  ScopedNativeStore(this.prefix);
  final String prefix;
  final native = SecureDeviceStore();
  final keys = <String>{};

  @override
  Future<String?> read(String key) => native.read('$prefix$key');
  @override
  Future<void> write(String key, String value) async {
    keys.add(key);
    await native.write('$prefix$key', value);
  }

  @override
  Future<void> delete(String key) => native.delete('$prefix$key');

  Future<void> clear() async {
    for (final key in keys) {
      await delete(key);
      expect(await read(key), isNull, reason: '試験値の削除をnative経路で確認する');
    }
  }
}
