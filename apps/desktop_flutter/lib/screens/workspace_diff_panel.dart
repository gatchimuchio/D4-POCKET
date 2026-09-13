import 'package:flutter/material.dart';

/// 描画前にWorkspaceClientが現在承認と差分構造を照合する。
class WorkspaceDiffPanel extends StatefulWidget {
  const WorkspaceDiffPanel({super.key, required this.diff});
  final Map<String, Object?> diff;
  @override
  State<WorkspaceDiffPanel> createState() => _WorkspaceDiffPanelState();
}

class _WorkspaceDiffPanelState extends State<WorkspaceDiffPanel> {
  bool _paired = false;
  static const _kinds = {
    'unchanged': '変更なし',
    'binary': 'バイナリ差分・本文非表示',
    'oversized': '差分上限超過・本文非表示',
    'text': 'テキスト差分'
  };
  static const _rows = {
    'same': '同一',
    'added': '追加',
    'deleted': '削除',
    'changed': '変更'
  };

  Widget _line(Object? raw, String kind, bool before) {
    if (raw == null) return const Expanded(child: Text('—'));
    final line = raw as Map;
    final text = line['text'] as String;
    final ending = line['newline'] == true
        ? (text.endsWith('\r') ? 'CRLF' : 'LF')
        : '改行なし';
    final changed = before
        ? kind == 'deleted' || kind == 'changed'
        : kind == 'added' || kind == 'changed';
    return Expanded(
        child: Container(
      padding: const EdgeInsets.all(6),
      color: changed
          ? (before
              ? Colors.red.withValues(alpha: 0.12)
              : Colors.green.withValues(alpha: 0.12))
          : null,
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Text('${line['number']}行・$ending'),
        SelectableText(text, style: const TextStyle(fontFamily: 'monospace')),
      ]),
    ));
  }

  @override
  Widget build(BuildContext context) {
    final diff = widget.diff;
    final rows = diff['rows'] as List;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Text(_kinds[diff['kind']]!),
      for (final side in ['before', 'after'])
        Text(
            '${side == 'before' ? '基準点' : '取得時点'}: ${diff[side] == null ? 'file不在' : '${(diff[side] as Map)['bytes']} バイト・${(diff[side] as Map)['sha256']}'}'),
      if (diff['kind'] == 'text') ...[
        Wrap(spacing: 8, children: [
          ChoiceChip(
              label: const Text('統合差分'),
              selected: !_paired,
              onSelected: (_) => setState(() => _paired = false)),
          ChoiceChip(
              label: const Text('左右比較'),
              selected: _paired,
              onSelected: (_) => setState(() => _paired = true)),
        ]),
        if (!_paired)
          SizedBox(
              height: 360,
              child: SingleChildScrollView(
                  child: SelectableText(diff['unified'] as String,
                      style: const TextStyle(fontFamily: 'monospace'))))
        else ...[
          const Row(children: [
            Expanded(child: Text('基準点')),
            Expanded(child: Text('取得時点'))
          ]),
          SizedBox(
              height: 360,
              child: ListView.builder(
                  itemCount: rows.length,
                  itemBuilder: (context, index) {
                    final row = rows[index] as Map;
                    final kind = row['kind'] as String;
                    return Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(_rows[kind]!),
                          Row(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                _line(row['before'], kind, true),
                                _line(row['after'], kind, false)
                              ]),
                        ]);
                  })),
        ],
      ],
    ]);
  }
}
