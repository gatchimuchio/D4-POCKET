import 'dart:async';

import 'package:flutter/material.dart';

import '../services/workspace_client.dart';
import 'workspace_diff_panel.dart';

class WorkspaceInspector extends StatefulWidget {
  const WorkspaceInspector({
    super.key,
    required this.client,
    this.runtimeId,
    this.workspaceId,
    this.active = true,
  });
  final WorkspaceClient client;
  final String? runtimeId;
  final String? workspaceId;
  final bool active;

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
  bool _diffPaired = false;
  String _requestedVisibility = 'hash_only';
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
      if (!_isActive || !mounted) return;
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
    if (_isActive) _refresh();
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
    if (oldWidget.client != widget.client ||
        oldWidget.runtimeId != widget.runtimeId ||
        oldWidget.workspaceId != widget.workspaceId) {
      _generation++;
      _view = null;
      _registrations = [];
      _selected = null;
      _refresh();
    } else if (oldWidget.active != widget.active) {
      _generation++;
      _view = null;
      _busy = false;
      _registrations = [];
      _selected = null;
      _message = '画面を離れたためWorkspace表示を消去しました。';
      if (_isActive) _refresh();
    }
  }

  bool get _isActive => _active && widget.active;

  @override
  void dispose() {
    _generation++;
    _timer?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  Future<void> _refresh({bool preserve = false}) async {
    if (!_isActive) return;
    final previous = preserve ? _view : null;
    final generation = ++_generation;
    setState(() {
      _view = null;
      _busy = true;
      _message = '現在の登録・承認状態を確認中';
    });
    try {
      final registrations = (await widget.client.list())
          .where((registration) =>
              (widget.runtimeId == null ||
                  registration.runtime == widget.runtimeId) &&
              (widget.workspaceId == null ||
                  registration.id == widget.workspaceId))
          .toList(growable: false);
      if (!mounted || !_isActive || generation != _generation) return;
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

      if (!mounted || !_isActive || generation != _generation) return;
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
    if (!_isActive) return;
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
      if (!mounted || !_isActive || generation != _generation) return;
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

  Future<void> _approveContent(WorkspaceRegistration registration) async {
    if (!_isActive || _busy) return;
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _view = null;
      _selected = registration.id;
      _message = 'native Owner確認待ちです。Workspace本文はまだ表示しません。';
    });
    try {
      final current = await widget.client
          .approveContent(registration, _requestedVisibility);
      if (!mounted || !_isActive || generation != _generation) return;
      setState(() {
        _registrations = _registrations
            .map((entry) => entry.id == current.id ? current : entry)
            .toList(growable: false);
        _message = '現在のWorkspace読取Approvalを確認しました。Task実行権は付与していません。';
      });
    } on Object {
      if (!mounted || generation != _generation) return;
      setState(() => _message = 'Workspace読取Approvalを確認できません。本文表示を停止しました。');
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _revokeContent(WorkspaceRegistration registration) async {
    if (!_isActive || _busy) return;
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _view = null;
      _message = 'native Owner確認待ちです。失効前の表示を消去しました。';
    });
    try {
      await widget.client.revokeContent(registration);
      if (!mounted || !_isActive || generation != _generation) return;
      setState(() {
        _registrations = _registrations
            .map((entry) => entry.id == registration.id
                ? WorkspaceRegistration.parse({
                    '作業領域ID': registration.id,
                    '実行系ID': registration.runtime,
                    '登録hash': registration.hash,
                    '承認状態': 'denied',
                    '有効期限': null,
                    '表示範囲': 'none',
                    'approval_id': null,
                  })
                : entry)
            .toList(growable: false);
        _message = 'Workspace読取ApprovalとBroker baselineを失効しました。';
      });
    } on Object {
      if (!mounted || generation != _generation) return;
      setState(() => _message = 'Workspace失効状態を確認できません。表示を停止しました。');
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _captureWholeBaseline(WorkspaceRegistration registration) async {
    if (!_isActive || _busy) return;
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _view = null;
      _selected = registration.id;
      _message = 'native Owner確認待ちです。Workspaceをまだ読み取りません。';
    });
    try {
      final view = await widget.client.captureWholeBaseline(registration);
      if (!mounted || !_isActive || generation != _generation) return;
      setState(() {
        _view = view;
        _viewLimit = _elapsed.elapsedMilliseconds + 300000;
        _message = 'Broker内の比較baselineを作成しました。Task実行・Workspace変更は行っていません。';
      });
    } on Object {
      if (!mounted || generation != _generation) return;
      setState(() => _message = 'Workspace baselineを作成できません。');
    } finally {
      if (mounted && generation == _generation) {
        setState(() {
          _busy = false;
          _nextCheck = _elapsed.elapsedMilliseconds + 2000;
        });
      }
    }
  }

  Future<void> _readBaselineChanges(WorkspaceView baseline) async {
    final hash = baseline.baselineHash;
    if (hash == null) return;
    await _read(baseline.registration, '', false,
        changes: true, baselineHash: hash);
  }

  @override
  Widget build(BuildContext context) {
    final view = _view;
    final projection = view?.projection;
    final selectedMatches =
        _registrations.where((registration) => registration.id == _selected);
    final selectedRegistration =
        selectedMatches.isEmpty ? null : selectedMatches.first;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Text('作業領域インスペクタ', style: Theme.of(context).textTheme.titleLarge),
      TextButton(
          onPressed: _busy || !_isActive ? null : () => _refresh(),
          child: const Text('登録状態を更新')),
      Text(_message),
      if (_busy) const LinearProgressIndicator(),
      for (final registration in _registrations)
        ListTile(
          title: Text(registration.id),
          subtitle: Text(
              '${registration.runtime}・${registration.visibility}・${registration.current(DateTime.now()) ? '承認あり' : '未承認または期限切れ'}'),
          selected: _selected == registration.id,
          onTap: !_isActive || _busy
              ? null
              : () {
                  setState(() {
                    _selected = registration.id;
                    _view = null;
                  });
                  if (registration.current(DateTime.now())) {
                    _read(registration, '', true);
                  }
                },
        ),
      if (selectedRegistration != null) ...[
        Text(
            '選択Workspace: ${selectedRegistration.id}・登録hash ${selectedRegistration.hash}'),
        DropdownButton<String>(
          value: _requestedVisibility,
          items: const [
            DropdownMenuItem(value: 'none', child: Text('none')),
            DropdownMenuItem(value: 'hash_only', child: Text('hash_only')),
            DropdownMenuItem(value: 'summary', child: Text('summary')),
            DropdownMenuItem(value: 'redacted', child: Text('redacted')),
            DropdownMenuItem(value: 'full', child: Text('full')),
          ],
          onChanged: !_isActive || _busy
              ? null
              : (value) {
                  if (value != null) {
                    setState(() => _requestedVisibility = value);
                  }
                },
        ),
        OutlinedButton(
          onPressed: _busy || !_isActive
              ? null
              : () => _approveContent(selectedRegistration),
          child: const Text('native Owner確認でWorkspace読取を許可'),
        ),
        if (selectedRegistration.current(DateTime.now())) ...[
          Text(
              '現在のWorkspace読取範囲: ${selectedRegistration.visibility}・Task実行／書込権は別です。'),
          OutlinedButton(
            onPressed: _busy || !_isActive
                ? null
                : () => _revokeContent(selectedRegistration),
            child: const Text('native Owner確認でWorkspace読取を失効'),
          ),
          if (selectedRegistration.visibility == 'full') ...[
            OutlinedButton(
              onPressed: _busy || !_isActive
                  ? null
                  : () => _read(selectedRegistration, '', false, scope: true),
              child: const Text('既存baselineの比較範囲を確認'),
            ),
            OutlinedButton(
              onPressed: _busy || !_isActive
                  ? null
                  : () => _captureWholeBaseline(selectedRegistration),
              child: const Text('native Owner確認で比較baselineを保存'),
            ),
          ],
        ],
      ],
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
        if (view.operation == '作業領域全体基準点保存') ...[
          Text('比較baseline hash: ${projection!['基準点hash']}'),
          Text('取得file数: ${projection['対象数']}'),
          const Text('Workspace単位のbaselineです。特定Taskの成果とはBroker上で結合されていません。'),
          OutlinedButton(
            onPressed: _busy ? null : () => _readBaselineChanges(view),
            child: const Text('baseline以降の変更file／差分を表示'),
          ),
        ],
        if (view.registration.visibility == 'full' &&
            view.operation == '作業領域比較範囲') ...[
          if (projection!['基準点hash'] == null)
            const Text('基準点がありません。所有者の制御操作で保存してください。')
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
