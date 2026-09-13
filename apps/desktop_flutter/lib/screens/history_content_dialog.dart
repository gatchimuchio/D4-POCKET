import 'dart:async';
import 'package:flutter/material.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';

/// 現在承認に限定した本文表示。owner承認を発行しない。
class HistoryContentDialog extends StatefulWidget {
  const HistoryContentDialog(
      {super.key, required this.client, required this.entry});
  final HistoryClient client;
  final HistoryEntry entry;
  @override
  State<HistoryContentDialog> createState() => _HistoryContentDialogState();
}

class _HistoryContentDialogState extends State<HistoryContentDialog>
    with WidgetsBindingObserver {
  HistoryContent? _content;
  Timer? _expiry, _poll;
  bool _checking = true, _active = true;
  int _generation = 0;
  String _message = '現在の内容閲覧承認を確認中';
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _expiry = Timer.periodic(const Duration(milliseconds: 100), (_) {
      if (mounted && _content != null && !_content!.grant.current) {
        _clear('閲覧期限を過ぎました。閉じて再承認してください。');
      }
    });
    _load();
  }

  void _clear(String message) {
    _generation++;
    _poll?.cancel();
    setState(() {
      _content = null;
      _checking = false;
      _message = message;
    });
  }

  Future<void> _load() async {
    final generation = ++_generation;
    try {
      final grant = await widget.client.contentStatus();
      if (grant == null || !grant.matches(widget.entry)) {
        throw const BrokerClientException('対象の内容閲覧承認が必要');
      }
      final content = await widget.client.content(widget.entry, grant);
      if (!mounted || !_active || generation != _generation) return;
      if (!grant.current) throw const BrokerClientException('閲覧期限超過');
      setState(() {
        _content = content;
        _checking = false;
        _message = '現在承認の期限内だけ表示します。';
      });
      _schedule();
    } catch (_) {
      if (mounted && generation == _generation) {
        _clear('対象の内容閲覧承認または保管内容を確認できません。');
      }
    }
  }

  void _schedule() {
    _poll = Timer(const Duration(seconds: 2), _check);
  }

  Future<void> _check() async {
    final content = _content;
    if (!mounted || !_active || content == null) return;
    final generation = _generation;
    setState(() => _checking = true);
    try {
      final grant = await widget.client.contentStatus();
      if (!mounted || !_active || generation != _generation) return;
      if (grant == null ||
          !content.grant.same(grant) ||
          !content.grant.current) {
        throw const BrokerClientException('承認失効');
      }
      setState(() => _checking = false);
      _schedule();
    } catch (_) {
      if (mounted && generation == _generation) {
        _clear('内容閲覧承認を再確認できないため表示を消去しました。');
      }
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _active = state == AppLifecycleState.resumed;
    _clear('画面状態が変わったため表示を消去しました。閉じて開き直してください。');
  }

  @override
  void dispose() {
    _generation++;
    _content = null;
    _expiry?.cancel();
    _poll?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final content = _checking ? null : _content;
    return AlertDialog(
      title: const Text('保存した対話内容'),
      content: SizedBox(
          width: 720,
          child: SingleChildScrollView(
              child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                Text(_checking ? '現在承認を確認中' : _message),
                if (content != null) ...[
                  const Text('入力'),
                  SelectableText(content.input),
                  const Divider(),
                  const Text('応答'),
                  SelectableText(content.result.text('本文')),
                  SelectableText(
                      '経路申告: ${content.result.text('経路')}\n能力申告: ${content.result.list('能力').join(', ')}\n追跡ID: ${content.result.text('追跡ID')}\n追跡hash: ${content.result.text('追跡hash')}'),
                  for (final reference in content.result.list('参照'))
                    SelectableText(reference),
                  const Text('保存時のAdapter申告です。現在の能力使用証明ではありません。'),
                ],
              ]))),
      actions: [
        TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('閉じる'))
      ],
    );
  }
}
