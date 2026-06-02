import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class AuditViewer extends StatefulWidget {
  const AuditViewer({super.key, required this.client});

  final ShellCoreClient client;

  @override
  State<AuditViewer> createState() => _AuditViewerState();
}

class _AuditViewerState extends State<AuditViewer> {
  final TextEditingController _searchController = TextEditingController();
  String _category = 'all';
  String _result = 'all';

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    final auditEvents = snapshot.auditEvents.where(_matchesFilters).toList();
    return ShellPage(
      title: 'Audit Viewer',
      children: [
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('Audit Timeline Filters',
                  style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              TextField(
                controller: _searchController,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.search),
                  labelText: 'Search audit events',
                ),
                onChanged: (_) => setState(() {}),
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  for (final category in [
                    'all',
                    'runtime',
                    'approval',
                    'recovery',
                    'evidence',
                    'setup_doctor',
                  ])
                    ChoiceChip(
                      label: Text(category),
                      selected: _category == category,
                      onSelected: (_) => setState(() => _category = category),
                    ),
                  const SizedBox(width: 8),
                  for (final result in ['all', 'success', 'warning', 'blocked'])
                    FilterChip(
                      label: Text(result),
                      selected: _result == result,
                      onSelected: (_) => setState(() => _result = result),
                    ),
                ],
              ),
            ],
          ),
        ),
        SectionList(
          title: 'Chain Status',
          rows: [
            'audit_chain_status: ${snapshot.auditChainStatus}',
            'hash_chain_status: ${snapshot.auditChainStatus}',
            'actions: copy event / export JSONL / verify chain / jump conceptually to related approval-runtime-adapter',
          ],
        ),
        if (auditEvents.isEmpty)
          const EmptyStatePanel(
            title: 'No audit events match',
            meaning: 'The current filters hide all audit events.',
            phaseBBlocked: false,
            nextAction: 'Clear filters or refresh diagnostics.',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('Event')),
                DataColumn(label: Text('Action')),
                DataColumn(label: Text('Result')),
                DataColumn(label: Text('Payload Hash')),
                DataColumn(label: Text('Previous')),
                DataColumn(label: Text('Related')),
                DataColumn(label: Text('Copy')),
              ],
              rows: [
                for (final event in auditEvents)
                  DataRow(cells: [
                    DataCell(Text(event.eventId)),
                    DataCell(Text(event.action)),
                    DataCell(Text(event.result)),
                    DataCell(Text(event.payloadHash)),
                    DataCell(Text(event.previousEventHash ?? 'root')),
                    DataCell(Text(_relatedText(snapshot, event))),
                    DataCell(
                      IconButton(
                        tooltip: 'Copy audit event JSON',
                        icon: const Icon(Icons.copy, size: 18),
                        onPressed: () => Clipboard.setData(
                          ClipboardData(text: _eventJson(event)),
                        ),
                      ),
                    ),
                  ]),
              ],
            ),
          ),
      ],
    );
  }

  bool _matchesFilters(AuditRecord event) {
    final searchable =
        '${event.eventId} ${event.action} ${event.result} ${event.payloadHash} ${event.previousEventHash ?? ''}'
            .toLowerCase();
    final query = _searchController.text.trim().toLowerCase();
    if (query.isNotEmpty && !searchable.contains(query)) {
      return false;
    }
    if (_category != 'all' && !searchable.contains(_category)) {
      return false;
    }
    if (_result != 'all' && !event.result.toLowerCase().contains(_result)) {
      return false;
    }
    return true;
  }
}

String _relatedText(ShellSnapshot snapshot, AuditRecord event) {
  final relatedAuthority = snapshot.authorityMap
      .where((item) => item.auditEventId == event.eventId)
      .toList();
  if (relatedAuthority.isEmpty) {
    return 'none';
  }
  return relatedAuthority
      .map(
          (item) => '${item.runtimeId}/${item.capabilityId}/${item.recoveryId}')
      .join('\n');
}

String _eventJson(AuditRecord event) {
  return const JsonEncoder.withIndent('  ').convert({
    'event_id': event.eventId,
    'action': event.action,
    'result': event.result,
    'payload_hash': event.payloadHash,
    'previous_event_hash': event.previousEventHash,
  });
}
