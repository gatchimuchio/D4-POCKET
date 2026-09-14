import 'package:gui_shell_ui/runtime_lifecycle_client.dart';

import 'broker_client.dart';

export 'package:gui_shell_ui/runtime_lifecycle_client.dart'
    show
        RuntimeLifecycleApproval,
        RuntimeLifecycleClient,
        RuntimeLifecycleGovernance,
        RuntimeLifecycleOperation,
        RuntimeLifecycleStatus,
        RuntimeLifecycleTransition;

typedef RuntimeLifecycleClientConnector = Future<RuntimeLifecycleClient>
    Function();

/// Desktopの通常資格でBroker所有のlifecycle表示・承認要求・実行要求だけを送る。
/// owner資格はこの層へ渡さず、owner CLIの承認経路に留める。
Future<RuntimeLifecycleClient> connectRuntimeLifecycleClient() async =>
    RuntimeLifecycleClient(await BrokerClient.connect());
