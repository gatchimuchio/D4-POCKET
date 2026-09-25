import 'package:gui_shell_ui/regression_case_client.dart';

import 'broker_client.dart';

export 'package:gui_shell_ui/regression_case_client.dart'
    show
        RegressionCaseClient,
        RegressionCaseDeleteReceipt,
        RegressionCaseOwnerClient,
        RegressionCasePage,
        RegressionCaseRegistrationReceipt,
        RegressionCaseRecoveryReceipt,
        RegressionCaseSummary;

typedef RegressionCaseClientConnector = Future<RegressionCaseClient> Function();

/// 通常Broker資格で、C6の公開metadata一覧だけを読むDesktop接続。
Future<RegressionCaseClient> connectRegressionCaseClient() async =>
    RegressionCaseClient(await BrokerClient.connect());

typedef RegressionCaseOwnerClientConnector = Future<RegressionCaseOwnerClient>
    Function();

/// 通常BrokerClientからallowlist要求を送る。権限はRust Desktop起動器の
/// Windows native確認とBroker内再検証を通過した要求にだけ付与される。
Future<RegressionCaseOwnerClient> connectRegressionCaseOwnerClient() async =>
    RegressionCaseOwnerClient(await BrokerClient.connect());
