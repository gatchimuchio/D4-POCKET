import copy

from .policy_evaluator import PolicyEvaluator
from .runtime_state import RuntimeState
from .error_taxonomy import SCHEMA_CONTRACT_MISSING, shell_error


class SensitiveActionRouter:
    def __init__(self, state: RuntimeState | None = None):
        self._evaluator = PolicyEvaluator(state) if state is not None else None

    def route(self, action: dict) -> dict:
        required = {
            "runtime_id",
            "operation",
            "capability_id",
            "permission_id",
            "approval_id",
            "target_scope",
            "audit_event",
            "recovery_action",
        }
        missing = sorted(required - set(action))
        if missing:
            raise ValueError(f"機密 action に必須の対応がありません: {', '.join(missing)}")
        routed = copy.deepcopy(action)
        if self._evaluator is None:
            routed["routed"] = False
            routed["policy_result"] = {
                "allowed": False,
                "errors": [
                    shell_error(
                        SCHEMA_CONTRACT_MISSING,
                        "製品用の機密 action router には evaluator と broker 所有 state が必要です",
                        routed.get("operation", "unknown"),
                    )
                ],
                "required_recovery": routed.get("recovery_action"),
                "audit_required": True,
            }
            return routed

        policy_result = self._evaluator.evaluate(routed)
        routed["policy_result"] = policy_result
        routed["routed"] = policy_result["allowed"]
        return routed
