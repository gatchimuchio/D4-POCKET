import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/services/agent_handoff.dart';
import 'package:gui_shell_desktop/services/agent_task_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException;

const _sourceSession = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _targetSession = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _sourceTask = 'cccccccccccccccccccccccccccccccc';
const _resultHash =
    'sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd';
const _differentHash =
    'sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff';

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

AgentTaskRecord _task({String status = 'completed'}) => AgentTaskRecord(
      taskId: _sourceTask,
      runtimeId: 'codex-a',
      sessionId: _sourceSession,
      workspaceId: 'workspace-a',
      instructionHash: _resultHash,
      status: status,
      auditEventId: 'audit-source-complete',
      resultHash: _resultHash,
      resultContentAvailable: true,
    );

AgentTaskResultProjection _result({
  String visibility = 'full',
  String? text,
}) =>
    AgentTaskResultProjection(
      taskId: _sourceTask,
      resultHash: _resultHash,
      visibility: visibility,
      projection: {
        'result_hash': _resultHash,
        'text': text ??
            jsonEncode({
              'result_summary': '実装を更新した',
              'artifacts': [
                {'name': 'patch.txt', 'content': 'diff --git ...'},
              ],
              'changed_files': ['lib/example.dart'],
              'diff': '+new line',
              'test_result': 'Agent申告: test passed（未検証）',
            }),
      },
    );

void main() {
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
        runtimeId: 'codex-b',
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
        runtimeId: 'codex-b',
        sessionId: _targetSession,
        workspaceId: 'workspace-b',
        instructionHash: _resultHash,
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
    }) =>
        AgentHandoffPackage.fromSource(
          source: _session(
            sessionId: _sourceSession,
            runtimeId: 'codex-a',
            workspace: 'workspace-a',
          ),
          task: task ?? _task(),
          taskContext: '公開文書を更新する',
          result: result ?? _result(),
          target: target ??
              _session(
                sessionId: _targetSession,
                runtimeId: 'codex-b',
                workspace: 'workspace-b',
              ),
        );

    expect(() => create(task: _task(status: 'running')),
        throwsA(isA<BrokerClientException>()));
    expect(() => create(result: _result(visibility: 'summary')),
        throwsA(isA<BrokerClientException>()));
    expect(
      () => create(
        result: const AgentTaskResultProjection(
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
          runtimeId: 'codex-a',
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
}
