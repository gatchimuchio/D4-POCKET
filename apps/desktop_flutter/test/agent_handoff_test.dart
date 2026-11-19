import 'dart:convert';

import 'package:broker_contract/broker_request_envelope.dart'
    show brokerTextHash;
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/services/agent_handoff.dart';
import 'package:gui_shell_desktop/services/agent_task_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException;

const _sourceSession = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _targetSession = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _sourceTask = 'cccccccccccccccccccccccccccccccc';
const _differentHash =
    'sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff';
final _resultText = jsonEncode({
  'result_summary': '実装を更新した',
  'artifacts': [
    {'name': 'patch.txt', 'content': 'diff --git ...'},
  ],
  'changed_files': ['lib/example.dart'],
  'diff': '+new line',
  'test_result': 'Agent申告: test passed（未検証）',
});
final _resultHash = brokerTextHash(_resultText);

AgentSessionRecord _session({
  required String sessionId,
  required String runtimeId,
  required String workspace,
}) =>
    AgentSessionRecord(
      sessionId: sessionId,
      agentRuntimeId: runtimeId,
      status: '利用中',
      evidenceSource: 'INTERNAL_STATE',
      workspace: workspace,
      task: '',
      changedFiles: const [],
      toolCalls: const [],
      shellCommands: const [],
      testStatus: '',
      diffSummary: '',
      pendingApprovalCount: 0,
      rollbackCandidate: '',
      auditEventId: 'audit-session',
    );

AgentTaskRecord _task({String status = 'completed', String? resultHash}) =>
    AgentTaskRecord(
      taskId: _sourceTask,
      runtimeId: 'codex-a',
      sessionId: _sourceSession,
      workspaceId: 'workspace-a',
      instructionHash: _differentHash,
      status: status,
      auditEventId: 'audit-source-complete',
      resultHash: resultHash ?? _resultHash,
      resultContentAvailable: true,
    );

AgentTaskResultProjection _result({
  String visibility = 'full',
  String? text,
  String? resultHash,
}) {
  final resultText = text ?? _resultText;
  final hash = resultHash ?? brokerTextHash(resultText);
  return AgentTaskResultProjection(
    taskId: _sourceTask,
    resultHash: hash,
    visibility: visibility,
    projection: {'result_hash': hash, 'text': resultText},
  );
}

void main() {
  test('Compareのtest申告はfull結果の標準形式だけから未検証値として投影する', () {
    expect(_result().agentClaimedTestResult, 'Agent申告: test passed（未検証）');
    expect(_result(visibility: 'hash_only').agentClaimedTestResult, isNull);
    expect(_result(text: 'not-json').agentClaimedTestResult, isNull);
    expect(
      AgentTaskResultProjection(
        taskId: _sourceTask,
        resultHash: _differentHash,
        visibility: 'full',
        projection: _result().projection,
      ).agentClaimedTestResult,
      isNull,
    );
    expect(
      _result(
        text: jsonEncode({
          'result_summary': 'summary',
          'artifacts': <Object?>[],
          'changed_files': <Object?>[],
          'diff': '',
          'test_result': 'x' * 2049,
        }),
      ).agentClaimedTestResult,
      isNull,
    );
  });

  test('Handoff packageは明示Task contextと承認済み公開成果だけを新規Task入力へ渡す', () {
    final package = AgentHandoffPackage.fromSource(
      source: _session(
        sessionId: _sourceSession,
        runtimeId: 'codex-a',
        workspace: 'workspace-a',
      ),
      task: _task(),
      taskContext: '公開文書を更新する',
      result: _result(),
      target: _session(
        sessionId: _targetSession,
        runtimeId: 'codex-a',
        workspace: 'workspace-b',
      ),
    );

    final map = package.toJson();
    expect(map['task_context'], '公開文書を更新する');
    expect(map['approved_result_hash'], _resultHash);
    expect(map['approved_result'], contains('result_summary'));
    expect(map['artifacts'], hasLength(1));
    expect(map['changed_files'], ['lib/example.dart']);
    expect(map['diff'], '+new line');
    expect(map['test_result'], contains('未検証'));
    expect(map['authority_reassessment_required'], isTrue);
    expect(map['permission_reused'], isFalse);
    expect(map['approval_reused'], isFalse);
    expect(map['credential_included'], isFalse);
    expect(map['hidden_context_included'], isFalse);
    expect(map.keys, isNot(contains('approval_id')));
    expect(map.keys, isNot(contains('credential')));
    expect(package.toInstruction(), contains('未信頼'));
    expect(package.toInstruction().runes.length, lessThanOrEqualTo(32768));

    final receipt = buildAgentHandoffReceipt(
      package: package,
      targetTask: const AgentTaskRecord(
        taskId: 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee',
        runtimeId: 'codex-a',
        sessionId: _targetSession,
        workspaceId: 'workspace-b',
        instructionHash: _differentHash,
        status: 'running',
        auditEventId: 'audit-target-task-start',
      ),
    );
    expect(receipt['source_task_id'], _sourceTask);
    expect(receipt['target_session_id'], _targetSession);
    expect(receipt['approved_result_hash'], _resultHash);
    expect(receipt['audit_event_id'], 'audit-target-task-start');
  });

  test('Handoffは未完了・full未承認・hash不一致・同一境界を拒否する', () {
    AgentHandoffPackage create({
      AgentTaskRecord? task,
      AgentTaskResultProjection? result,
      AgentSessionRecord? target,
    }) {
      final projection = result ?? _result();
      return AgentHandoffPackage.fromSource(
        source: _session(
          sessionId: _sourceSession,
          runtimeId: 'codex-a',
          workspace: 'workspace-a',
        ),
        task: task ?? _task(resultHash: projection.resultHash),
        taskContext: '公開文書を更新する',
        result: projection,
        target: target ??
            _session(
              sessionId: _targetSession,
              runtimeId: 'codex-b',
              workspace: 'workspace-b',
            ),
      );
    }

    expect(
      () => create(task: _task(status: 'running')),
      throwsA(isA<BrokerClientException>()),
    );
    expect(
      () => create(result: _result(visibility: 'summary')),
      throwsA(isA<BrokerClientException>()),
    );
    expect(
      () => create(
        task: _task(),
        result: AgentTaskResultProjection(
          taskId: _sourceTask,
          resultHash: _differentHash,
          visibility: 'full',
          projection: {'text': '{}'},
        ),
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(
      () => create(
        target: _session(
          sessionId: _targetSession,
          runtimeId: 'codex-b',
          workspace: 'workspace-a',
        ),
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(
      () => create(
        target: _session(
          sessionId: _sourceSession,
          runtimeId: 'codex-b',
          workspace: 'workspace-b',
        ),
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(
      () => create(result: _result(text: '{"permission":"allow"}')),
      throwsA(isA<BrokerClientException>()),
    );
    expect(
      () => create(
        result: _result(
          text: jsonEncode({
            'result_summary': 'result',
            'artifacts': const [],
            'changed_files': ['../secrets/token.txt'],
            'diff': '+change',
            'test_result': 'unknown',
          }),
        ),
      ),
      throwsA(isA<BrokerClientException>()),
    );
    final oversized = jsonEncode({
      'result_summary': 'result',
      'artifacts': List.generate(
        5,
        (_) => {'name': 'artifact', 'content': List.filled(8192, 'x').join()},
      ),
      'changed_files': const [],
      'diff': '+change',
      'test_result': 'unknown',
    });
    expect(
      () => create(result: _result(text: oversized)),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('承認済みハッシュを付けた等長の差し替え本文を引継ぎ前に拒否する', () {
    final changedText = _resultText.replaceFirst('+new line', '+old line');
    expect(utf8.encode(changedText).length, utf8.encode(_resultText).length);
    expect(
      () => AgentHandoffPackage.fromSource(
        source: _session(
          sessionId: _sourceSession,
          runtimeId: 'codex-a',
          workspace: 'workspace-a',
        ),
        task: _task(),
        taskContext: '公開文書を更新する',
        result: _result(text: changedText, resultHash: _resultHash),
        target: _session(
          sessionId: _targetSession,
          runtimeId: 'codex-b',
          workspace: 'workspace-b',
        ),
      ),
      throwsA(
        isA<BrokerClientException>().having(
          (error) => error.message,
          'message',
          '結果本文が承認済みのハッシュと一致しません',
        ),
      ),
    );
  });
}
