import 'dart:async';

import 'package:flutter/material.dart';

import '../services/workspace_client.dart';

class WorkspaceInspector extends StatefulWidget {
  const WorkspaceInspector({super.key, required this.client});
  final WorkspaceClient client;

  @override
  State<WorkspaceInspector> createState() => _WorkspaceInspectorState();
}

class _WorkspaceInspectorState extends State<WorkspaceInspector>
    with WidgetsBindingObserver {
  List<WorkspaceRegistration> _registrations = [];
  WorkspaceView? _view;
  String? _selected;
  String _message = '登録済み作業領域を確認中';
  bool _busy = false;
  bool _active = true;
  int _generation = 0;
  Timer? _timer;
  final _elapsed = Stopwatch()..start();
  int _nextCheck = 0;
  int _viewLimit = 0;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _timer = Timer.periodic(const Duration(milliseconds: 100), (_) {
      if (!_active || !mounted) return;
      if (_view != null &&
          (!_view!.registration.current(DateTime.now()) ||
              _elapsed.elapsedMilliseconds >= _viewLimit)) {
        _generation++;
        setState(() {
          _view = null;
          _busy = false;
          _message = '表示期限を過ぎました。登録状態を更新してください。';
        });
      }
      if (_view != null &&
          !_busy &&
          _elapsed.elapsedMilliseconds >= _nextCheck) {
        _refresh(preserve: true);
      }
    });
    _refresh();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _active = state == AppLifecycleState.resumed;
    _generation++;
    setState(() {
      _view = null;
      _busy = false;
      _registrations = [];
      _message = '画面復帰後は登録状態を更新してください。';
    });
  }

  @override
  void didUpdateWidget(covariant WorkspaceInspector oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.client != widget.client) {
      _generation++;
      _view = null;
      _registrations = [];
      _selected = null;
      _refresh();
    }
  }

  @override
  void dispose() {
    _generation++;
    _timer?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  Future<void> _refresh({bool preserve = false}) async {
    final previous = preserve ? _view : null;
    final generation = ++_generation;
    setState(() {
      _view = null;
      _busy = true;
      _message = '現在の登録・承認状態を確認中';
    });
    try {
      final registrations = await widget.client.list();
      if (!mounted || !_active || generation != _generation) return;
      setState(() {
        _registrations = registrations;
        if (!registrations.any((v) => v.id == _selected)) _selected = null;
        if (previous != null &&
            previous.registration.current(DateTime.now()) &&
            _elapsed.elapsedMilliseconds < _viewLimit &&
            registrations.any((v) => v.sameGrant(previous.registration))) {
          _view = previous;
        }
        _message = registrations.isEmpty
            ? '登録済み作業領域はありません。'
            : '読取には所有者による現在登録の承認が必要です。';
      });
    } catch (_) {
      if (!mounted || generation != _generation) return;
      setState(() {
        _registrations = [];
        _view = null;
        _message = '登録状態を確認できないため表示を停止しました。';
      });
    } finally {
      if (mounted && generation == _generation) {
        setState(() {
          _busy = false;
          _nextCheck = _elapsed.elapsedMilliseconds + 2000;
        });
      }
    }
  }

  Future<void> _read(
      WorkspaceRegistration registration, String path, bool tree) async {
    final generation = ++_generation;
    setState(() {
      _view = null;
      _busy = true;
      _selected = registration.id;
      _message = '承認済み範囲を取得中';
    });
    try {
      final view = await widget.client.read(registration, path, tree: tree);
      if (!mounted || !_active || generation != _generation) return;
      setState(() {
        _view = view;
        _viewLimit = _elapsed.elapsedMilliseconds + 300000;
        _message = '取得時点の表示です。承認状態を定期確認しています。';
      });
    } catch (_) {
      if (!mounted || generation != _generation) return;
      setState(() {
        _view = null;
        _message = '取得または現在の承認を確認できません。登録状態を更新してください。';
      });
    } finally {
      if (mounted && generation == _generation) {
        setState(() {
          _busy = false;
          _nextCheck = _elapsed.elapsedMilliseconds + 2000;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final view = _view;
    final projection = view?.projection;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Text('作業領域インスペクタ', style: Theme.of(context).textTheme.titleLarge),
      TextButton(
          onPressed: _busy || !_active ? null : () => _refresh(),
          child: const Text('登録状態を更新')),
      Text(_message),
      if (_busy) const LinearProgressIndicator(),
      for (final registration in _registrations)
        ListTile(
          title: Text(registration.id),
          subtitle: Text(
              '${registration.runtime}・${registration.visibility}・${registration.current(DateTime.now()) ? '承認あり' : '未承認または期限切れ'}'),
          selected: _selected == registration.id,
          onTap: !_active || _busy || !registration.current(DateTime.now())
              ? null
              : () => _read(registration, '', true),
        ),
      if (view != null) ...[
        Text('監査: ${view.auditId}'),
        TextButton(
            onPressed: _busy ? null : () => _read(view.registration, '', true),
            child: const Text('作業領域の先頭へ')),
        if (view.registration.visibility == 'none') const Text('内容は非表示です。'),
        if (view.registration.visibility == 'hash_only')
          SelectableText(projection!['sha256'] as String),
        if (view.registration.visibility == 'summary' ||
            view.registration.visibility == 'redacted')
          Text(projection!['説明'] as String),
        if (view.registration.visibility == 'full' &&
            view.operation == '作業領域ツリー')
          for (final item in projection!['entries'] as List)
            ListTile(
              leading: Icon(item['kind'] == 'directory'
                  ? Icons.folder
                  : Icons.description),
              title: Text(item['path'] as String),
              subtitle: Text('${item['bytes']} バイト'),
              onTap: _busy
                  ? null
                  : () => _read(view.registration, item['path'] as String,
                      item['kind'] == 'directory'),
            ),
        if (view.registration.visibility == 'full' &&
            view.operation == '作業領域読取') ...[
          Text(view.path),
          Text('${projection!['bytes']} バイト・${projection['sha256']}'),
          if (projection['binary'] == true)
            const Text('バイナリのため本文は表示しません。')
          else
            SelectableText(projection['text'] as String),
        ],
      ],
    ]);
  }
}
