import 'dart:async';

import 'package:flutter/foundation.dart' show ValueListenable;
import 'package:flutter/material.dart';

import '../services/ai_edit_client.dart';
import '../services/workspace_client.dart';
import 'workspace_diff_panel.dart';

class WorkspaceInspectorHostScope extends InheritedWidget {
  const WorkspaceInspectorHostScope({
    super.key,
    required this.generation,
    required super.child,
  });

  final ValueListenable<int> generation;

  @override
  bool updateShouldNotify(WorkspaceInspectorHostScope oldWidget) =>
      !identical(generation, oldWidget.generation);
}

class WorkspaceInspector extends StatefulWidget {
  const WorkspaceInspector({
    super.key,
    required this.client,
    this.runtimeId,
    this.workspaceId,
    this.editProposal,
    this.active = true,
  });
  final WorkspaceClient client;
  final String? runtimeId;
  final String? workspaceId;
  final AiEditProposalReview? editProposal;
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
  final Set<String> _expandedReviewPaths = {};
  String _requestedVisibility = 'hash_only';
  int _generation = 0;
  int _bindingGeneration = 0;
  ValueListenable<int>? _hostContextGeneration;
  bool _hostContextInitialized = false;
  Timer? _viewMaintenanceTimer;
  final _elapsed = Stopwatch()..start();
  int _nextCheck = 0;
  int _viewLimit = 0;

  String _failureMessage(Object error, String fallback) =>
      error is WorkspaceClientFailure ? error.userMessage : fallback;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final generation = context
        .dependOnInheritedWidgetOfExactType<WorkspaceInspectorHostScope>()
        ?.generation;
    if (_hostContextInitialized &&
        identical(generation, _hostContextGeneration)) {
      return;
    }
    _hostContextGeneration?.removeListener(_hostContextChanged);
    _hostContextGeneration = generation;
    generation?.addListener(_hostContextChanged);
    if (_hostContextInitialized) {
      _clearForHostContextChange();
      return;
    }
    _hostContextInitialized = true;
    if (_isActive) _refresh();
  }

  void _hostContextChanged() {
    if (mounted) setState(_clearForHostContextChange);
  }

  void _clearForHostContextChange() {
    _clearBinding('Hostが切り替わったため作業領域の表示を消去しました。登録状態を更新してください。');
  }

  void _clearBinding(String message) {
    _bindingGeneration++;
    _generation++;
    _cancelViewMaintenance();
    _view = null;
    _registrations = [];
    _selected = null;
    _busy = false;
    _diffPaired = false;
    _expandedReviewPaths.clear();
    _requestedVisibility = 'hash_only';
    _message = message;
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _active = state == AppLifecycleState.resumed;
    _generation++;
    _cancelViewMaintenance();
    setState(() {
      _view = null;
      _busy = false;
      _registrations = [];
      _message = '画面復帰後は登録状態を更新してください。';
    });
    if (_isActive) _refresh();
  }

  @override
  void didUpdateWidget(covariant WorkspaceInspector oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.client != widget.client ||
        oldWidget.runtimeId != widget.runtimeId ||
        oldWidget.workspaceId != widget.workspaceId ||
        oldWidget.editProposal != widget.editProposal) {
      _clearBinding('作業領域の接続先または対象が変わりました。登録状態を更新してください。');
      _refresh();
    } else if (oldWidget.active != widget.active) {
      _cancelViewMaintenance();
      _generation++;
      _view = null;
      _busy = false;
      _registrations = [];
      _message = '画面を離れたためWorkspace表示を消去しました。';
      if (_isActive) _refresh();
    }
  }

  bool get _isActive => _active && widget.active;

  @override
  void dispose() {
    _generation++;
    _cancelViewMaintenance();
    _hostContextGeneration?.removeListener(_hostContextChanged);
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  void _cancelViewMaintenance() {
    _viewMaintenanceTimer?.cancel();
    _viewMaintenanceTimer = null;
  }

  void _scheduleViewMaintenance() {
    _cancelViewMaintenance();
    final view = _view;
    if (!_isActive || _busy || view == null) return;

    final now = DateTime.now();
    final elapsed = _elapsed.elapsedMilliseconds;
    if (!view.registration.current(now) ||
        view.sourceRegistration?.current(now) == false ||
        elapsed >= _viewLimit) {
      _expireView();
      return;
    }

    var delayMilliseconds = _nextCheck - elapsed;
    final viewRemaining = _viewLimit - elapsed;
    if (viewRemaining < delayMilliseconds) {
      delayMilliseconds = viewRemaining;
    }
    final registrationRemaining =
        view.registration.expires! * 1000 - now.millisecondsSinceEpoch;
    if (registrationRemaining < delayMilliseconds) {
      delayMilliseconds = registrationRemaining;
    }
    final sourceExpires = view.sourceRegistration?.expires;
    if (sourceExpires != null) {
      final sourceRemaining = sourceExpires * 1000 - now.millisecondsSinceEpoch;
      if (sourceRemaining < delayMilliseconds) {
        delayMilliseconds = sourceRemaining;
      }
    }
    if (delayMilliseconds < 1) delayMilliseconds = 1;

    _viewMaintenanceTimer = Timer(
      Duration(milliseconds: delayMilliseconds),
      _maintainView,
    );
  }

  void _maintainView() {
    _viewMaintenanceTimer = null;
    if (!mounted || !_isActive || _busy) return;
    final view = _view;
    if (view == null) return;

    final now = DateTime.now();
    if (!view.registration.current(now) ||
        view.sourceRegistration?.current(now) == false ||
        _elapsed.elapsedMilliseconds >= _viewLimit) {
      _expireView();
      return;
    }
    if (_elapsed.elapsedMilliseconds >= _nextCheck) {
      _refresh(preserve: true);
      return;
    }
    _scheduleViewMaintenance();
  }

  void _expireView() {
    if (!mounted || _view == null) return;
    _cancelViewMaintenance();
    _generation++;
    setState(() {
      _view = null;
      _busy = false;
      _message = '表示期限を過ぎました。登録状態を更新してください。';
    });
  }

  Future<void> _refresh({bool preserve = false}) async {
    if (!_isActive) return;
    _cancelViewMaintenance();
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
        final source = previous?.sourceRegistration;
        final now = DateTime.now();
        _registrations = registrations;
        if (!registrations.any((v) => v.id == _selected)) _selected = null;
        if (previous != null &&
            previous.registration.current(now) &&
            (source == null ||
                (source.current(now) &&
                    registrations.any((v) => v.sameGrant(source)))) &&
            _elapsed.elapsedMilliseconds < _viewLimit &&
            registrations.any((v) => v.sameGrant(previous.registration))) {
          _view = refreshedComparison ?? previous;
        }
        _message = registrations.isEmpty
            ? '登録済み作業領域はありません。'
            : '読取には所有者による現在登録の承認が必要です。';
      });
    } catch (error) {
      if (!mounted || generation != _generation) return;
      setState(() {
        _registrations = [];
        _view = null;
        _message = _failureMessage(
          error,
          '登録状態を確認できないため表示を停止しました。Broker接続と現在状態を確認してください。',
        );
      });
    } finally {
      if (mounted && generation == _generation) {
        setState(() {
          _busy = false;
          _nextCheck = _elapsed.elapsedMilliseconds + 2000;
        });
        _scheduleViewMaintenance();
      }
    }
  }

  Future<void> _read(WorkspaceRegistration registration, String path, bool tree,
      {bool scope = false,
      bool changes = false,
      bool preview = false,
      bool review = false,
      WorkspaceView? prepareTargetFor,
      WorkspaceView? applyTarget,
      String? baselineHash}) async {
    if (!_isActive) return;
    _cancelViewMaintenance();
    final generation = ++_generation;
    setState(() {
      _diffPaired = false;
      _expandedReviewPaths.clear();
      _view = null;
      _busy = true;
      _selected = registration.id;
      _message = applyTarget == null ? '承認済み範囲を取得中' : 'Owner確認と適用の結果を待っています。';
    });
    try {
      final WorkspaceView view;
      if (applyTarget != null) {
        view = await widget.client.applyReviewedEdit(applyTarget);
      } else if (prepareTargetFor != null) {
        view = await widget.client.prepareEditTarget(prepareTargetFor, registration);
      } else if (review) {
        view = await widget.client.reviewEditProposal(
            registration, widget.editProposal!, baselineHash!);
      } else {
        view = await widget.client.read(registration, path,
              tree: tree,
              scope: scope,
              changes: changes,
              preview: preview,
              baselineHash: baselineHash);
      }
      if (!mounted || !_isActive || generation != _generation) return;
      setState(() {
        _view = view;
        _viewLimit = _elapsed.elapsedMilliseconds + 300000;
        _message = applyTarget == null
            ? '取得時点の表示です。承認状態を定期確認しています。'
            : '審査済みの差分を適用しました。';
      });
    } catch (error) {
      if (!mounted || generation != _generation) return;
      setState(() {
        _view = null;
        _message = _failureMessage(
          error,
          applyTarget == null
              ? '取得または現在の承認を確認できません。登録状態を更新してください。'
              : '適用結果を確認できません。再実行の前に監査と現在の差分を確認してください。',
        );
      });
    } finally {
      if (mounted && generation == _generation) {
        setState(() {
          _busy = false;
          _nextCheck = _elapsed.elapsedMilliseconds + 2000;
        });
        _scheduleViewMaintenance();
      }
    }
  }

  Future<void> _selectEditTarget(WorkspaceView review) async {
    final now = DateTime.now();
    final targets = _registrations.where((candidate) =>
        candidate.id != review.registration.id &&
        candidate.visibility == 'full' &&
        candidate.current(now));
    final target = await showDialog<WorkspaceRegistration>(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('適用先Workspaceを選択'),
        children: [
          for (final registration in targets)
            SimpleDialogOption(
              onPressed: () => Navigator.pop(context, registration),
              child: Text(registration.id),
            ),
          if (targets.isEmpty)
            const Padding(
              padding: EdgeInsets.all(16),
              child: Text('適用先Workspaceにfull読取承認が必要です。'),
            ),
        ],
      ),
    );
    if (!mounted || !_isActive || _busy || _view != review || target == null) {
      return;
    }
    await _read(target, '', false, prepareTargetFor: review);
  }

  Future<void> _approveContent(WorkspaceRegistration registration) async {
    if (!_isActive || _busy) return;
    _cancelViewMaintenance();
    final generation = ++_generation;
    final bindingGeneration = _bindingGeneration;
    setState(() {
      _busy = true;
      _view = null;
      _selected = registration.id;
      _message = 'native Owner確認待ちです。Workspace本文はまだ表示しません。';
    });
    try {
      final current = await widget.client
          .approveContent(registration, _requestedVisibility);
      if (!mounted || bindingGeneration != _bindingGeneration) return;
      if (generation != _generation) {
        // Owner確認中の画面切替で一覧取得と承認応答が競合した場合、
        // 選択中Workspaceが同じときだけBrokerの現在状態を再取得する。
        if (_isActive && _selected == registration.id) {
          await _refresh();
        }
        return;
      }
      if (!_isActive) return;
      setState(() {
        _registrations = _registrations
            .map((entry) => entry.id == current.id ? current : entry)
            .toList(growable: false);
        _message = '現在のWorkspace読取Approvalを確認しました。Task実行権は付与していません。';
      });
    } on Object catch (error) {
      if (!mounted || bindingGeneration != _bindingGeneration) return;
      if (generation != _generation) {
        // native Owner確認中に画面が非activeになった後で要求が失敗しても、
        // Brokerが確定した現在状態だけを再取得する。Approvalは再利用・再発行しない。
        if (_isActive && _selected == registration.id) {
          await _refresh();
        }
        return;
      }
      setState(() => _message = _failureMessage(
            error,
            'Workspace読取Approvalを確認できません。本文表示を停止しました。',
          ));
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
      }
    }
  }

  void _selectRegistration(WorkspaceRegistration registration) {
    if (!_isActive || _busy) return;
    setState(() {
      _selected = registration.id;
      _view = null;
    });
    if (registration.current(DateTime.now())) {
      _read(registration, '', true);
    }
  }

  Future<void> _revokeContent(WorkspaceRegistration registration) async {
    if (!_isActive || _busy) return;
    _cancelViewMaintenance();
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
    } on Object catch (error) {
      if (!mounted || generation != _generation) return;
      setState(() => _message = _failureMessage(
            error,
            'Workspace失効状態を確認できません。登録状態を更新して現在の承認を確認してください。',
          ));
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _captureWholeBaseline(WorkspaceRegistration registration) async {
    if (!_isActive || _busy) return;
    _cancelViewMaintenance();
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
    } on Object catch (error) {
      if (!mounted || generation != _generation) return;
      setState(() => _message = _failureMessage(
            error,
            'Workspace baselineを作成できません。Broker状態と現在の承認を確認してください。',
          ));
    } finally {
      if (mounted && generation == _generation) {
        setState(() {
          _busy = false;
          _nextCheck = _elapsed.elapsedMilliseconds + 2000;
        });
        _scheduleViewMaintenance();
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
    return Semantics(
        container: true,
        explicitChildNodes: true,
        label: '作業領域インスペクタ',
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Semantics(
          header: true,
          label: '作業領域インスペクタ',
          child: Text('作業領域インスペクタ',
              style: Theme.of(context).textTheme.titleLarge)),
      TextButton(
          onPressed: _busy || !_isActive ? null : () => _refresh(),
          child: const Text('登録状態を更新')),
      Text(_message),
      if (_busy) const LinearProgressIndicator(),
      for (final registration in _registrations)
        Semantics(
          container: true,
          label:
              '${registration.id}・${registration.runtime}・${registration.visibility}・${registration.current(DateTime.now()) ? '承認あり' : '未承認または期限切れ'}',
          button: true,
          selected: _selected == registration.id,
          enabled: _isActive && !_busy,
          onTap: !_isActive || _busy
              ? null
              : () => _selectRegistration(registration),
          child: ExcludeSemantics(
            child: ListTile(
              title: Text(registration.id),
              subtitle: Text(
                  '${registration.runtime}・${registration.visibility}・${registration.current(DateTime.now()) ? '承認あり' : '未承認または期限切れ'}'),
              selected: _selected == registration.id,
              onTap: !_isActive || _busy
                  ? null
                  : () => _selectRegistration(registration),
            ),
          ),
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
        if (widget.editProposal != null &&
            view.registration.visibility == 'full' &&
            projection?['基準点hash'] is String &&
            (view.operation == '作業領域比較範囲' ||
                view.operation == '作業領域全体基準点保存'))
          OutlinedButton(
            onPressed: _busy
                ? null
                : () => _read(view.registration, '', false,
                    review: true,
                    baselineHash: projection!['基準点hash'] as String),
            child: const Text('提案範囲と実差分を照合'),
          ),
        if (view.operation == workspaceEditReviewOperation) ...[
          Text('審査差分 ${projection!['diff_hash']}'),
          Text('読取除外 ${projection['excluded_secrets']}件'),
          OutlinedButton(
            onPressed: _busy ? null : () => _selectEditTarget(view),
            child: const Text('適用先を照合'),
          ),
          for (final item in projection['changes'] as List)
            ExpansionTile(
              key: ValueKey('${view.auditId}:${item['path']}'),
              title: Text(item['path'] as String),
              initiallyExpanded: _expandedReviewPaths.contains(item['path']),
              onExpansionChanged: (expanded) {
                final path = item['path'] as String;
                if (expanded) {
                  _expandedReviewPaths.add(path);
                } else {
                  _expandedReviewPaths.remove(path);
                }
              },
              children: [
                WorkspaceDiffPanel(
                  diff: Map<String, Object?>.from(item['diff'] as Map),
                  paired: _diffPaired,
                  onPairedChanged: (value) =>
                      setState(() => _diffPaired = value),
                ),
              ],
            ),
        ],
        if (view.operation == workspaceEditApplyTargetOperation) ...[
          Text('審査元 ${view.sourceRegistration!.id}'),
          Text('適用先 ${view.registration.id}'),
          Text('審査差分 ${projection!['diff_hash']}'),
          Text('変更前後の版を照合した対象 ${(projection['files'] as List).length}件'),
          OutlinedButton(
            onPressed: _busy
                ? null
                : () => _read(view.registration, '', false, applyTarget: view),
            child: const Text('Owner確認して適用'),
          ),
        ],
        if (view.operation == workspaceEditApplyOperation) ...[
          Text('適用済み ${projection!['changed_files']}件'),
          Text('復旧記録 ${projection['transaction_id']}'),
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
    ]));
  }
}
