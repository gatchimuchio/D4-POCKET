"""C28検証器の一時Broker資格file後始末の回帰試験。"""

from pathlib import Path
import unittest
from unittest.mock import call, patch

from tooling import long_run_validation as c28


class RestartEndpointCleanupTests(unittest.TestCase):
    def test_permission_denial_is_retried_and_counted(self):
        target = Path("broker.json")
        with (
            patch.object(c28, "C28_ENDPOINT_UNLINK_MAX_ATTEMPTS", 3),
            patch.object(Path, "unlink", autospec=True) as unlink,
            patch.object(c28.time, "sleep") as sleep,
        ):
            unlink.side_effect = [PermissionError(13, "sharing"), PermissionError(13, "sharing"), None]

            retries = c28.unlink_restart_endpoint(target)

        self.assertEqual(retries, 2)
        self.assertEqual(unlink.call_count, 3)
        self.assertEqual(
            sleep.call_args_list,
            [call(c28.C28_ENDPOINT_UNLINK_RETRY_SECONDS)] * 2,
        )

    def test_persistent_permission_denial_remains_a_failure(self):
        target = Path("broker.json")
        with (
            patch.object(c28, "C28_ENDPOINT_UNLINK_MAX_ATTEMPTS", 3),
            patch.object(Path, "unlink", autospec=True, side_effect=PermissionError(13, "denied")) as unlink,
            patch.object(c28.time, "sleep") as sleep,
        ):
            with self.assertRaises(PermissionError):
                c28.unlink_restart_endpoint(target)

        self.assertEqual(unlink.call_count, 3)
        self.assertEqual(sleep.call_count, 2)

    def test_missing_endpoint_is_idempotent(self):
        with patch.object(Path, "unlink", autospec=True, side_effect=FileNotFoundError):
            self.assertEqual(c28.unlink_restart_endpoint(Path("missing.json")), 0)

    def test_restart_cleanup_accounts_for_retries_without_logging_paths(self):
        run = object.__new__(c28.LongRun)
        run.session = Path("broker.json")
        run.owner_session = Path("owner.json")
        run.stats = c28.Statistics()
        with patch.object(c28, "unlink_restart_endpoint", side_effect=[2, 1]) as unlink:
            run._paths_for_restart()

        self.assertEqual(unlink.call_args_list, [call(run.session), call(run.owner_session)])
        self.assertEqual(run.stats.endpoint_cleanup_retries, 3)
        self.assertEqual(run.stats.as_dict()["Broker資格file削除再試行数"], 3)


if __name__ == "__main__":
    unittest.main()
