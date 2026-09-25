"""C28検証器のcleanupと診断境界の回帰試験。"""

from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import call, Mock, patch

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

    def _http_handler(self):
        state = c28.RuntimeState()
        handler = object.__new__(c28.RuntimeHandler)
        handler.path = "/health"
        handler.server = SimpleNamespace(runtime_state=state)
        handler.send_response = Mock()
        handler.send_header = Mock()
        handler.end_headers = Mock()
        handler.wfile = Mock()
        return handler, state

    def test_fixture_status_is_recorded_after_response_flush(self):
        handler, state = self._http_handler()
        events = []
        handler.wfile.write.side_effect = lambda _body: events.append("write")
        handler.wfile.flush.side_effect = lambda: events.append("flush")
        original_record = state.record_http_result

        def record(path, status, detail=""):
            events.append("record")
            original_record(path, status, detail)

        with (
            patch.object(state, "record_http_result", side_effect=record),
            patch.object(c28.time, "sleep"),
        ):
            handler._send({"ok": True})

        self.assertEqual(events, ["write", "flush", "record"])
        self.assertEqual(state.http_results[0]["detail"], "server_flush_succeeded")

    def test_fixture_write_failure_records_only_error_class(self):
        handler, state = self._http_handler()
        handler.wfile.write.side_effect = BrokenPipeError("private detail must not be copied")
        with (
            self.assertRaises(BrokenPipeError),
            patch.object(c28.time, "sleep"),
        ):
            handler._send({"ok": True})

        self.assertEqual(
            state.http_results,
            [{"path": "health", "status": 200, "detail": "server_write_failed:BrokenPipeError"}],
        )

    def test_fixture_telemetry_is_bounded_and_uses_route_labels(self):
        state = c28.RuntimeState()
        state.record_http("/health")
        state.record_http("/api/capabilities")
        state.record_http("/api/chat")
        first_trace_id = None
        latest_trace_id = None
        for index in range(c28.MAX_FIXTURE_TRACES + 8):
            trace_id, _ = state.next_trace(f"試験セッション-{index}", "検証要求")
            first_trace_id = first_trace_id or trace_id
            latest_trace_id = trace_id
            state.record_http(f"/api/trace/{trace_id}")
            state.record_http_result(f"/api/trace/{trace_id}", 200, "server_flush_succeeded")
        state.record_http("/unrecognized/path/with/機密値001")
        state.record_http_result("/unrecognized/path/with/機密値001", 404)

        snapshot = state.http_snapshot()
        self.assertEqual(len(state.traces), c28.MAX_FIXTURE_TRACES)
        self.assertNotIn(first_trace_id, state.traces)
        self.assertIn(latest_trace_id, state.traces)
        self.assertEqual(len(state.http_results), c28.MAX_FIXTURE_HTTP_RESULTS)
        self.assertLessEqual(len(snapshot["paths"]), 5)
        self.assertEqual(snapshot["paths"]["trace"], c28.MAX_FIXTURE_TRACES + 8)
        self.assertEqual(snapshot["paths"]["other"], 1)
        self.assertEqual(snapshot["request_count"], c28.MAX_FIXTURE_TRACES + 12)
        self.assertEqual({item["path"] for item in snapshot["results"]}, {"trace", "other"})
        self.assertTrue(all("/api/trace/" not in item["path"] for item in snapshot["results"]))
        self.assertTrue(all("機密値001" not in item["path"] for item in snapshot["results"]))
        self.assertNotIn("0" * 32, str(snapshot))

    def test_transport_diagnostics_are_allowlisted_and_bounded(self):
        lines = [
            "C28_RUNTIME_DIAGNOSTIC|chat|read_body|ConnectionReset",
            "C28_RUNTIME_DIAGNOSTIC|trace|read_headers|TimedOut",
            "C28_RUNTIME_DIAGNOSTIC|chat|read_body|private-session-secret",
            "C28_RUNTIME_DIAGNOSTIC|../secret|read_body|ConnectionReset",
            "unrelated stderr containing private-session-secret",
        ]
        lines.extend(
            f"C28_RUNTIME_DIAGNOSTIC|chat|read_body|ConnectionReset" for _ in range(40)
        )

        safe = c28.safe_c28_diagnostics("\n".join(lines))

        self.assertEqual(len(safe), c28.C28_DIAGNOSTIC_LIMIT)
        self.assertTrue(all("private-session-secret" not in line for line in safe))
        self.assertTrue(all("../secret" not in line for line in safe))
        self.assertEqual(safe[0], "C28_RUNTIME_DIAGNOSTIC|chat|read_body|ConnectionReset")

    def test_error_report_omits_exception_message(self):
        error = RuntimeError("CREDENTIAL_SENTINEL_001")

        safe = c28.safe_c28_error_type(error)
        code = c28.safe_c28_failure_code(error)

        self.assertEqual(safe, "RuntimeError")
        self.assertEqual(code, "unexpected_error")
        self.assertNotIn("CREDENTIAL_SENTINEL_001", safe)
        self.assertNotIn("CREDENTIAL_SENTINEL_001", code)

    def test_fixed_dialogue_failure_code_omits_exception_message(self):
        error = c28.C28ValidationError(
            "dialogue_result_not_success", "PRIVATE_DIALOGUE_SENTINEL"
        )

        self.assertEqual(
            c28.safe_c28_failure_code(error), "dialogue_result_not_success"
        )
        self.assertNotIn(
            "PRIVATE_DIALOGUE_SENTINEL", c28.safe_c28_failure_code(error)
        )

    def test_unlisted_failure_code_is_rejected(self):
        with self.assertRaises(ValueError):
            c28.C28ValidationError("PRIVATE_SENTINEL", "private message")

    def test_diagnostics_are_scoped_to_the_current_offset(self):
        import tempfile

        earlier = b"C28_RUNTIME_DIAGNOSTIC|chat|read_headers|ConnectionReset\n"
        current = b"C28_RUNTIME_DIAGNOSTIC|health|connect|TimedOut\n"
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "broker.stderr"
            path.write_bytes(earlier + current)

            diagnostics = c28.safe_c28_diagnostics_since(path, len(earlier))

        self.assertEqual(
            diagnostics,
            ["C28_RUNTIME_DIAGNOSTIC|health|connect|TimedOut"],
        )


if __name__ == "__main__":
    unittest.main()
