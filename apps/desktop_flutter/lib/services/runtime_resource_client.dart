import 'package:gui_shell_ui/runtime_resource_client.dart';

import 'broker_client.dart';

export 'package:gui_shell_ui/runtime_resource_client.dart'
    show
        RuntimeResourceBinding,
        RuntimeResourceClient,
        RuntimeResourceGovernance,
        RuntimeResourceHistorySample,
        RuntimeResourceMetric,
        RuntimeResourceObservation;

typedef RuntimeResourceClientConnector = Future<RuntimeResourceClient>
    Function();

/// Desktopの認証済みloopback Brokerへ、資源観測時だけ接続する。
Future<RuntimeResourceClient> connectRuntimeResourceClient() async =>
    RuntimeResourceClient(await BrokerClient.connect());
