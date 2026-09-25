import 'package:gui_shell_ui/regression_case_client.dart';

import 'broker_client.dart';

export 'package:gui_shell_ui/regression_case_client.dart'
    show RegressionCaseClient, RegressionCasePage, RegressionCaseSummary;

typedef RegressionCaseClientConnector = Future<RegressionCaseClient> Function();

/// 通常Broker資格で、C6の公開metadata一覧だけを読むDesktop接続。
Future<RegressionCaseClient> connectRegressionCaseClient() async =>
    RegressionCaseClient(await BrokerClient.connect());
