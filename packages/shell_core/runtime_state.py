import copy

from .adapter_loader import load_adapter
from .schema_validation import validate_contract


class RuntimeState:
    def __init__(self):
        self.runtimes: dict[str, dict] = {}
        self.adapters: dict[str, dict] = {}
        self.capabilities: dict[str, dict] = {}
        self.permissions: dict[str, dict] = {}
        self.approvals: dict[str, dict] = {}
        self.audit_events: dict[str, dict] = {}
        self.recovery_actions: dict[str, dict] = {}
        self.update_policies: dict[str, dict] = {}

    def register_runtime(self, runtime: dict) -> None:
        validate_contract(runtime, "runtime.schema.json")
        self.runtimes[runtime["runtime_id"]] = copy.deepcopy(runtime)

    def register_adapter(self, adapter: dict) -> None:
        validate_contract(adapter, "adapter.schema.json")
        record = load_adapter(adapter)
        self.adapters[record.adapter_id] = {
            "adapter_id": record.adapter_id,
            "runtime_id": record.runtime_id,
            "contract_version": record.contract_version,
            "transport": record.transport,
            "authority_strip": True,
            "declared_capabilities": list(record.declared_capabilities),
            "metadata": copy.deepcopy(record.metadata),
        }

    def register_capability(self, capability: dict) -> None:
        validate_contract(capability, "capability.schema.json")
        self.capabilities[capability["capability_id"]] = copy.deepcopy(capability)

    def record_permission(self, permission: dict) -> None:
        validate_contract(permission, "permission.schema.json")
        self.permissions[permission["permission_id"]] = copy.deepcopy(permission)

    def enqueue_approval(self, approval: dict) -> None:
        validate_contract(approval, "approval.schema.json")
        self.approvals[approval["approval_id"]] = copy.deepcopy(approval)

    def append_audit_event(self, audit_event: dict) -> None:
        validate_contract(audit_event, "audit.schema.json")
        event_id = audit_event["event_id"]
        if event_id in self.audit_events:
            raise ValueError(f"重複した audit event_id です: {event_id}")
        self.audit_events[event_id] = copy.deepcopy(audit_event)

    def register_recovery_action(self, recovery_action: dict) -> None:
        validate_contract(recovery_action, "recovery.schema.json")
        self.recovery_actions[recovery_action["recovery_id"]] = copy.deepcopy(recovery_action)

    def register_update_policy(self, update_policy: dict) -> None:
        validate_contract(update_policy, "update.schema.json")
        self.update_policies[update_policy["policy_id"]] = copy.deepcopy(update_policy)

    def pending_approvals(self) -> list[dict]:
        return [
            copy.deepcopy(self.approvals[key])
            for key in sorted(self.approvals)
            if self.approvals[key].get("status") in {"pending", "requires_validation"}
        ]

    def clone(self) -> "RuntimeState":
        cloned = RuntimeState()
        cloned.runtimes = copy.deepcopy(self.runtimes)
        cloned.adapters = copy.deepcopy(self.adapters)
        cloned.capabilities = copy.deepcopy(self.capabilities)
        cloned.permissions = copy.deepcopy(self.permissions)
        cloned.approvals = copy.deepcopy(self.approvals)
        cloned.audit_events = copy.deepcopy(self.audit_events)
        cloned.recovery_actions = copy.deepcopy(self.recovery_actions)
        cloned.update_policies = copy.deepcopy(self.update_policies)
        return cloned
