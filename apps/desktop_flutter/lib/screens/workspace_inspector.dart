import 'dart:async';

import 'package:flutter/material.dart';

import '../services/workspace_client.dart';
import 'workspace_diff_panel.dart';

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
  final Map<String, String> _requestedVisibility = {};
  String _message = '登録済み作業領域を確認中';
  bool _busy = false;
  bool _active = true;
  bool _diffPaired = false;
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
      WorkspaceView? refreshedComparison;
      if (previous != null &&
          (previous.operation == '作業領域復旧プレビュー' ||
              previous.operation == '作業領域差分' ||
              previous.operation == '作業領域比較範囲' ||
              previous.operation == '作業領域変更一覧') &&
          registrations.any((v) => v.sameGrant(previous.registration))) {
        refreshedComparison = await widget.client.read(
            previous.registration, previous.path,
            tree: false,
            scope: previous.operation == '作業領域比較範囲',
            changes: previous.operation == '作業領域変更一覧',
            preview: previous.operation == '作業領域復旧プレビュー',
            baselineHash: previous.baselineHash);
      }

      if (!mounted || !_active || generation != _generation) return;
      setState(() {
        _registrations = registrations;
        if (!registrations.any((v) => v.id == _selected)) _selected = null;
        if (previous != null &&
            previous.registration.current(DateTime.now()) &&
            _elapsed.elapsedMilliseconds < _viewLimit &&
            registrations.any((v) => v.sameGrant(previous.registration))) {
          _view = refreshedComparison ?? previous;
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

  Future<void> _read(WorkspaceRegistration registration, String path, bool tree,
      {bool scope = false,
      bool changes = false,
      bool preview = false,
      String? baselineHash}) async {
    final generation = ++_generation;
    setState(() {
      _diffPaired = false;
      _view = null;
      _busy = true;
      _selected = registration.id;
      _message = '承認済み範囲を取得中';
    });
    try {
      final view = await widget.client.read(registration, path,
          tree: tree,
          scope: scope,
          changes: changes,
          preview: preview,
          baselineHash: baselineHash);
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

  String _requestedVisibilityFor(WorkspaceRegistration registration) =>
      _requestedVisibility['${registration.id}:${registration.hash}'] ??
      (registration.current(DateTime.now())
          ? registration.visibility
          : 'summary');

  Future<void> _approveRead(WorkspaceRegistration registration) async {
    final generation = ++_generation;
    final visibility = _requestedVisibilityFor(registration);
    setState(() {
      _view = null;
      _busy = true;
      _selected = registration.id;
      _message = 'native Owner確認で読取範囲を要求中';
    });
    try {
      await widget.client.approveRead(registration, visibility);
      final registrations = await widget.client.list();
      if (!mounted || !_active || generation != _generation) return;
      WorkspaceRegistration? granted;
      for (final item in registrations) {
        if (item.id == registration.id &&
            item.hash == registration.hash &&
            item.visibility == visibility &&
            item.current(DateTime.now())) {
          granted = item;
          break;
        }
      }
      setState(() {
        _registrations = registrations;
        _busy = false;
        _message = granted == null
            ? 'Brokerが現在の読取承認を返さないため表示を停止しました。'
            : 'Brokerの現在登録・承認を再確認しました。';
      });
      if (granted != null) {
        await _read(granted, '', visibility != 'full',
            scope: visibility == 'full');
      }
    } catch (_) {
      if (!mounted || generation != _generation) return;
      setState(() {
        _busy = false;
        _view = null;
        _message = 'Owner確認またはBroker承認が成立しないため表示を停止しました。';
      });
    }
  }

  Future<void> _saveWholeBaseline(WorkspaceRegistration registration) async {
    final generation = ++_generation;
    setState(() {
      _view = null;
      _busy = true;
      _message = 'native Owner確認後、Brokerが全体基準点を取得します';
    });
    try {
      await widget.client.saveWholeBaseline(registration);
      if (!mounted || !_active || generation != _generation) return;
      setState(() {
        _busy = false;
        _message = 'Brokerが全体基準点を保存しました。範囲を再取得中です。';
      });
      await _read(registration, '', false, scope: true);
    } catch (_) {
      if (!mounted || generation != _generation) return;
      setState(() {
        _busy = false;
        _view = null;
        _message = '基準点保存または現在の承認を確認できません。表示を停止しました。';
      });
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
        Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          ListTile(
            title: Text(registration.id),
            subtitle: Text(
                '${registration.runtime}・${registration.visibility}・${registration.current(DateTime.now()) ? '承認あり' : '未承認または期限切れ'}'),
            selected: _selected == registration.id,
            onTap: !_active || _busy || !registration.current(DateTime.now())
                ? null
                : () => _read(registration, '', true),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Wrap(
              spacing: 12,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                const Text('Owner承認範囲'),
                DropdownButton<String>(
                  value: _requestedVisibilityFor(registration),
                  items: const [
                    DropdownMenuItem(value: 'none', child: Text('非表示 (none)')),
                    DropdownMenuItem(
                        value: 'hash_only', child: Text('hashのみ (hash_only)')),
                    DropdownMenuItem(
                        value: 'summary', child: Text('概要 (summary)')),
                    DropdownMenuItem(
                        value: 'redacted', child: Text('redacted')),
                    DropdownMenuItem(
                        value: 'full', child: Text('本文・差分 (full)')),
                  ],
                  onChanged: !_active || _busy
                      ? null
                      : (value) {
                          if (value == null) return;
                          setState(() => _requestedVisibility[
                                  '${registration.id}:${registration.hash}'] =
                              value);
                        },
                ),
                TextButton.icon(
                  onPressed: !_active || _busy
                      ? null
                      : () => _approveRead(registration),
                  icon: const Icon(Icons.admin_panel_settings),
                  label: Text(registration.current(DateTime.now())
                      ? '読取範囲を変更・再承認'
                      : '読取範囲を承認'),
                ),
              ],
            ),
          ),
        ]),
      if (view != null) ...[
        Text('監査: ${view.auditId}'),
        TextButton(
            onPressed: _busy
                ? null
                : () => _read(view.registration, '', false, scope: true),
            child: const Text('基準点の対象file')),
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
            view.operation == '作業領域比較範囲') ...[
          if (projection!['基準点hash'] == null)
            Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              const Text('基準点がありません。保存には現在のfull読取承認とnative Owner確認が必要です。'),
              TextButton.icon(
                  onPressed: _busy
                      ? null
                      : () => _saveWholeBaseline(view.registration),
                  icon: const Icon(Icons.save_alt),
                  label: const Text('全体基準点を保存')),
            ])
          else ...[
            const Text('基準点に保存されたfileです。全体基準点では変更一覧も取得できます。'),
            TextButton(
                onPressed: _busy
                    ? null
                    : () => _read(view.registration, '', false,
                        changes: true,
                        baselineHash: projection['基準点hash'] as String),
                child: const Text('全体基準点の変更一覧')),
            Text('対象 ${(projection['相対paths'] as List).length}件'),
            SizedBox(
                height: 320,
                child: ListView.builder(
                    itemCount: (projection['相対paths'] as List).length,
                    itemBuilder: (context, index) {
                      final path =
                          (projection['相対paths'] as List)[index] as String;
                      return ListTile(
                          title: Text(path),
                          trailing: const Icon(Icons.compare_arrows),
                          onTap: _busy
                              ? null
                              : () => _read(view.registration, path, false,
                                  baselineHash:
                                      projection['基準点hash'] as String));
                    })),
          ],
        ],
        if (view.registration.visibility == 'full' &&
            view.operation == '作業領域変更一覧') ...[
          Text(
              '変更 ${(projection!['changes'] as List).length}件・変更なし ${projection['unchanged']}件・secret除外 ${projection['excluded_secrets']}件'),
          const Text('取得時点の比較です。secret配下は対象外です。'),
          SizedBox(
              height: 320,
              child: ListView.builder(
                  itemCount: (projection['changes'] as List).length,
                  itemBuilder: (context, index) {
                    final item = (projection['changes'] as List)[index] as Map;
                    final label = {
                      'added': '追加',
                      'modified': '変更',
                      'deleted': '削除'
                    }[item['status']]!;
                    return ListTile(
                        title: Text(item['path'] as String),
                        subtitle: Text(label),
                        trailing: const Icon(Icons.compare_arrows),
                        onTap: _busy
                            ? null
                            : () => _read(view.registration,
                                item['path'] as String, false,
                                baselineHash: view.baselineHash));
                  })),
        ],
        if (view.registration.visibility == 'full' &&
            (view.operation == '作業領域差分' ||
                view.operation == '作業領域復旧プレビュー')) ...[
          Text(view.path),
          if (view.operation == '作業領域差分')
            TextButton(
                onPressed: _busy
                    ? null
                    : () => _read(view.registration, view.path, false,
                        preview: true, baselineHash: view.baselineHash),
                child: const Text('復旧プレビュー')),
          if (view.operation == '作業領域復旧プレビュー') ...[
            Text('復旧候補: ${{
              'none': '変更なし',
              'remove': 'file削除',
              'recreate': 'file再作成',
              'replace': '内容置換'
            }[projection!['action']]}'),
            const Text('現在の内容 → 復旧先の基準点。適用には別のRecoveryとApprovalが必要です。'),
            const Text('この画面から復旧は実行できません。'),
            if (projection['baseline_content_available'] == false)
              const Text('比較元の本文を保持していないため、この基準点だけでは本文を復元できません。'),
          ],
          WorkspaceDiffPanel(
              reverse: view.operation == '作業領域復旧プレビュー',
              paired: _diffPaired,
              onPairedChanged: (value) => setState(() => _diffPaired = value),
              key: ValueKey('${view.path}:${projection!['基準点hash']}'),
              diff: Map<String, Object?>.from(projection['diff'] as Map)),
        ],
        if (view.registration.visibility == 'full' &&
            view.operation == '作業領域ツリー')
          for (final item in projection!['entries'] as List)
            ListTile(
              leading: Icon(item['kind'] == 'directory'
                  ? Icons.folder
                  : Icons.description),
              title: Text(item['path'] as String),
              subtitle: Text(item['kind'] == 'directory'
                  ? 'フォルダー'
                  : '${item['bytes']} バイト'),
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
