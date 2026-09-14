//! Windows実行系のread-only資源観測をBroker内へ閉じ込める。
//!
//! このmoduleはadapterのloopback listenerをOS側でPIDへ照合し、各観測で同じendpointの
//! 所有者とprocess作成時刻を再照合する。PIDをIPC入力として受け取らず、PID再利用、
//! listener消失、別processへの差替え時に別processへ追従しない。観測値は権限、Approval、
//! 実行可能性、runtime healthを生成しない。
#![allow(non_snake_case)]

use std::collections::{BTreeMap, VecDeque};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Instant;

use serde_json::{json, Value};

use super::dialogue::実行系資源統計;

const MAX_HISTORY: usize = 60;
const WINDOWS_UNIX_EPOCH_OFFSET_100NS: u64 = 116_444_736_000_000_000;
const MAX_EXACT_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Default)]
pub(crate) struct RuntimeResourceRegistry {
    entries: BTreeMap<String, RuntimeRegistration>,
}

#[derive(Debug)]
struct RuntimeRegistration {
    registered_at_millis: i64,
    registration_audit_id: String,
    binding: RuntimeBinding,
    previous_sample: Option<CpuSample>,
    history: VecDeque<Value>,
}

#[derive(Debug, Clone)]
struct RuntimeBinding {
    status: &'static str,
    basis: &'static str,
    target: Option<SocketAddrV4>,
    pid: Option<u32>,
    creation_time_100ns: Option<u64>,
    creation_time_unix_millis: Option<i64>,
    reason: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
struct CpuSample {
    total_cpu_100ns: u64,
    sampled_at: Instant,
}

#[derive(Debug, Clone, Copy)]
struct ProcessSample {
    creation_time_100ns: u64,
    total_cpu_100ns: u64,
    working_set_bytes: u64,
    private_bytes: u64,
    sampled_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceObservationError {
    UnknownRuntime,
}

pub(crate) struct RuntimeResourceObservation {
    pub(crate) body: Value,
    pub(crate) evidence_source: &'static str,
}

impl RuntimeResourceRegistry {
    pub(crate) fn register(
        &mut self,
        runtime_id: &str,
        target: Option<SocketAddr>,
        registered_at_millis: i64,
        registration_audit_id: String,
    ) {
        let binding = bind_at_registration(target);
        self.entries.insert(
            runtime_id.to_owned(),
            RuntimeRegistration {
                registered_at_millis,
                registration_audit_id,
                binding,
                previous_sample: None,
                history: VecDeque::new(),
            },
        );
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    /// terminal lifecycle隔離後に通常資源観測を継続させない。
    pub(crate) fn unregister(&mut self, runtime_id: &str) {
        self.entries.remove(runtime_id);
    }

    pub(crate) fn observe(
        &mut self,
        runtime_id: &str,
        stats: 実行系資源統計,
        observed_at_millis: i64,
    ) -> Result<RuntimeResourceObservation, ResourceObservationError> {
        let Some(entry) = self.entries.get_mut(runtime_id) else {
            return Err(ResourceObservationError::UnknownRuntime);
        };

        if entry.binding.status != "bound" {
            return Ok(unknown_observation(runtime_id, entry, observed_at_millis));
        }

        let Some(pid) = entry.binding.pid else {
            invalidate_binding(entry, "binding_mismatch", "PID結合情報が欠落しています");
            return Ok(unknown_observation(runtime_id, entry, observed_at_millis));
        };
        let Some(expected_creation) = entry.binding.creation_time_100ns else {
            invalidate_binding(entry, "binding_mismatch", "PID作成時刻の結合情報が欠落しています");
            return Ok(unknown_observation(runtime_id, entry, observed_at_millis));
        };

        let sample = match capture_current_binding(entry, pid, expected_creation) {
            Ok(sample) => sample,
            Err(CurrentBindingError::Exited(reason)) => {
                invalidate_binding(entry, "exited", reason);
                return Ok(unknown_observation(runtime_id, entry, observed_at_millis));
            }
            Err(CurrentBindingError::Mismatch(reason)) => {
                invalidate_binding(entry, "binding_mismatch", reason);
                return Ok(unknown_observation(runtime_id, entry, observed_at_millis));
            }
        };

        let uptime_millis = filetime_to_unix_millis(sample.creation_time_100ns)
            .and_then(|created_at| observed_at_millis.checked_sub(created_at))
            .filter(|value| *value >= 0)
            .map(|value| value as u64);
        let cpu_percent = entry.previous_sample.and_then(|previous| {
            let elapsed_100ns = sample
                .sampled_at
                .checked_duration_since(previous.sampled_at)?
                .as_nanos()
                .checked_div(100)?;
            let logical_processors = std::thread::available_parallelism()
                .ok()
                .and_then(|count| u64::try_from(count.get()).ok())
                .filter(|count| *count != 0)?;
            cpu_percent_from_raw(
                sample.total_cpu_100ns,
                previous.total_cpu_100ns,
                elapsed_100ns,
                logical_processors,
            )
        });
        entry.previous_sample = Some(CpuSample {
            total_cpu_100ns: sample.total_cpu_100ns,
            sampled_at: sample.sampled_at,
        });

        let metrics = measured_metrics(sample, uptime_millis, cpu_percent, stats);
        let history = json!({
            "観測時刻UnixMillis": observed_at_millis,
            "CPU利用率Percent": metrics["CPU利用率Percent"].clone(),
            "RAMWorkingSetBytes": metrics["RAMWorkingSetBytes"].clone(),
            "NetworkIOBytes": metrics["NetworkIOBytes"].clone(),
            "平均応答Millis": metrics["平均応答Millis"].clone(),
            "エラー率Percent": error_rate_metric(stats),
        });
        entry.history.push_back(history);
        while entry.history.len() > MAX_HISTORY {
            entry.history.pop_front();
        }

        Ok(RuntimeResourceObservation {
            body: json!({
                "版": 1,
                "実行系ID": runtime_id,
                "観測時刻UnixMillis": observed_at_millis,
                "結合": binding_json(entry),
                "統治": governance_json(),
                "計測": metrics,
                "短期履歴": entry.history.iter().cloned().collect::<Vec<_>>(),
            }),
            evidence_source: "LIVE_RUNTIME",
        })
    }
}

fn bind_at_registration(target: Option<SocketAddr>) -> RuntimeBinding {
    #[cfg(windows)]
    {
        let Some(target) = target else {
            return unbound("アダプターがloopback観測対象を提供していません");
        };
        let SocketAddr::V4(target) = target else {
            return unbound("アダプター観測対象が明示IPv4 loopbackではありません");
        };
        if *target.ip() != Ipv4Addr::LOCALHOST || target.port() == 0 {
            return unbound("アダプター観測対象が明示IPv4 loopbackではありません");
        }
        let (second_pid, second_sample) = match verify_registration_binding(
            || listener_owner_pid(target),
            capture_process,
        ) {
            Ok(binding) => binding,
            Err(RegistrationBindingError::Unbound(reason)) => return unbound(reason),
            Err(RegistrationBindingError::Exited(reason)) => {
                return RuntimeBinding {
                    status: "exited",
                    basis: "loopback_tcp_listener_owner_pid",
                    target: None,
                    pid: None,
                    creation_time_100ns: None,
                    creation_time_unix_millis: None,
                    reason: Some(reason),
                }
            }
        };
        let Some(creation_time_unix_millis) = filetime_to_unix_millis(second_sample.creation_time_100ns) else {
            return unbound("process作成時刻をUnix時刻へ安全に変換できませんでした");
        };
        return RuntimeBinding {
            status: "bound",
            basis: "loopback_tcp_listener_owner_pid",
            target: Some(target),
            pid: Some(second_pid),
            creation_time_100ns: Some(second_sample.creation_time_100ns),
            creation_time_unix_millis: Some(creation_time_unix_millis),
            reason: None,
        };
    }
    #[cfg(not(windows))]
    {
        let _ = target;
        RuntimeBinding {
            status: "unsupported",
            basis: "unsupported",
            target: None,
            pid: None,
            creation_time_100ns: None,
            creation_time_unix_millis: None,
            reason: Some("このOSではWindows実行系資源観測を実装していません"),
        }
    }
}

#[cfg(windows)]
fn listener_owner_pid(target: SocketAddrV4) -> Result<Option<u32>, ()> {
    gui_shell_windows_runtime_observation::loopback_tcp_listener_owner_pid(target)
        .map_err(|_| ())
}

enum RegistrationBindingError {
    Unbound(&'static str),
    Exited(&'static str),
}

/// 登録時に保存する有限binding snapshotを、endpoint所有者とprocess sampleの両側から固定する。
/// sample取得中のlistener移動を最後のowner照合で検出し、未確定snapshotをregistryへ残さない。
fn verify_registration_binding<Owner, Sample>(
    mut listener_owner: Owner,
    mut process_sample: Sample,
) -> Result<(u32, ProcessSample), RegistrationBindingError>
where
    Owner: FnMut() -> Result<Option<u32>, ()>,
    Sample: FnMut(u32) -> Result<ProcessSample, ()>,
{
    let first_pid = match listener_owner() {
        Ok(Some(pid)) => pid,
        Ok(None) => {
            return Err(RegistrationBindingError::Unbound(
                "登録時にloopback listenerを確認できませんでした",
            ))
        }
        Err(()) => {
            return Err(RegistrationBindingError::Unbound(
                "loopback listenerの所有processを一意に確認できませんでした",
            ))
        }
    };
    let first_sample = process_sample(first_pid).map_err(|_| {
        RegistrationBindingError::Exited("listener所有processを読取専用で確認できませんでした")
    })?;

    let second_pid = match listener_owner() {
        Ok(Some(pid)) => pid,
        Ok(None) => {
            return Err(RegistrationBindingError::Unbound(
                "登録中にloopback listenerが消失しました",
            ))
        }
        Err(()) => {
            return Err(RegistrationBindingError::Unbound(
                "登録中にloopback listenerの所有processを再照合できませんでした",
            ))
        }
    };
    if first_pid != second_pid {
        return Err(RegistrationBindingError::Unbound(
            "登録中にloopback listenerの所有processが変化しました",
        ));
    }
    let second_sample = process_sample(second_pid).map_err(|_| {
        RegistrationBindingError::Exited("登録中にlistener所有processを再確認できませんでした")
    })?;
    if first_sample.creation_time_100ns != second_sample.creation_time_100ns {
        return Err(RegistrationBindingError::Unbound(
            "登録中にlistener所有processの作成時刻が変化しました",
        ));
    }

    let third_pid = match listener_owner() {
        Ok(Some(pid)) => pid,
        Ok(None) => {
            return Err(RegistrationBindingError::Unbound(
                "登録中のprocess読取後にloopback listenerが消失しました",
            ))
        }
        Err(()) => {
            return Err(RegistrationBindingError::Unbound(
                "登録中のprocess読取後にloopback listenerを再照合できませんでした",
            ))
        }
    };
    if third_pid != second_pid {
        return Err(RegistrationBindingError::Unbound(
            "登録中のprocess読取後にloopback listenerの所有processが変化しました",
        ));
    }
    Ok((second_pid, second_sample))
}

enum CurrentBindingError {
    Exited(&'static str),
    Mismatch(&'static str),
}

/// 観測時点にも、登録済みendpointの所有者が同じprocessであることを複数段階で確認する。
/// 返すprocess sampleはowner照合の前後に置く。別owner、listener消失、PID再利用を
/// 検出した場合は値を返さない。
#[cfg(windows)]
fn capture_current_binding(
    entry: &RuntimeRegistration,
    expected_pid: u32,
    expected_creation: u64,
) -> Result<ProcessSample, CurrentBindingError> {
    let target = entry
        .binding
        .target
        .ok_or(CurrentBindingError::Mismatch("登録済みloopback endpointが欠落しています"))?;
    verify_current_binding_sample(
        expected_pid,
        expected_creation,
        || listener_owner_pid(target),
        capture_process,
    )
}

fn verify_current_binding_sample<Owner, Sample>(
    expected_pid: u32,
    expected_creation: u64,
    mut listener_owner: Owner,
    mut process_sample: Sample,
) -> Result<ProcessSample, CurrentBindingError>
where
    Owner: FnMut() -> Result<Option<u32>, ()>,
    Sample: FnMut(u32) -> Result<ProcessSample, ()>,
{
    let first_owner = listener_owner().map_err(|_| {
        CurrentBindingError::Mismatch("loopback listenerの所有processを再照合できません")
    })?;
    if first_owner != Some(expected_pid) {
        return Err(CurrentBindingError::Mismatch(
            "登録済みloopback endpointの所有processが一致しません",
        ));
    }
    let first_sample = process_sample(expected_pid).map_err(|_| {
        CurrentBindingError::Exited("登録済みprocessを再確認できません")
    })?;
    if first_sample.creation_time_100ns != expected_creation {
        return Err(CurrentBindingError::Mismatch(
            "PID再利用またはprocess生成時刻の不一致を検出しました",
        ));
    }

    let second_owner = listener_owner().map_err(|_| {
        CurrentBindingError::Mismatch("process読取後にloopback listenerを再照合できません")
    })?;
    if second_owner != Some(expected_pid) {
        return Err(CurrentBindingError::Mismatch(
            "process読取後にloopback endpointの所有processが変化しました",
        ));
    }
    let second_sample = process_sample(expected_pid).map_err(|_| {
        CurrentBindingError::Exited("二回目のprocess読取で登録済みprocessを確認できません")
    })?;
    if second_sample.creation_time_100ns != expected_creation {
        return Err(CurrentBindingError::Mismatch(
            "二回目のprocess読取でPID再利用または作成時刻不一致を検出しました",
        ));
    }

    let third_owner = listener_owner().map_err(|_| {
        CurrentBindingError::Mismatch("返却前にloopback listenerを再照合できません")
    })?;
    if third_owner != Some(expected_pid) {
        return Err(CurrentBindingError::Mismatch(
            "返却前にloopback endpointの所有processが変化しました",
        ));
    }
    Ok(second_sample)
}

#[cfg(not(windows))]
fn capture_current_binding(
    _entry: &RuntimeRegistration,
    _expected_pid: u32,
    _expected_creation: u64,
) -> Result<ProcessSample, CurrentBindingError> {
    Err(CurrentBindingError::Mismatch(
        "このOSではWindows実行系資源観測を実装していません",
    ))
}

fn unbound(reason: &'static str) -> RuntimeBinding {
    RuntimeBinding {
        status: "unbound",
        basis: "loopback_tcp_listener_owner_pid",
        target: None,
        pid: None,
        creation_time_100ns: None,
        creation_time_unix_millis: None,
        reason: Some(reason),
    }
}

fn invalidate_binding(entry: &mut RuntimeRegistration, status: &'static str, reason: &'static str) {
    entry.binding.status = status;
    entry.binding.target = None;
    entry.binding.pid = None;
    entry.binding.creation_time_100ns = None;
    entry.binding.creation_time_unix_millis = None;
    entry.binding.reason = Some(reason);
    entry.previous_sample = None;
    entry.history.clear();
}

fn unknown_observation(
    runtime_id: &str,
    entry: &RuntimeRegistration,
    observed_at_millis: i64,
) -> RuntimeResourceObservation {
    let reason = entry.binding.reason.unwrap_or("実行系processの結合を確認できませんでした");
    RuntimeResourceObservation {
        body: json!({
            "版": 1,
            "実行系ID": runtime_id,
            "観測時刻UnixMillis": observed_at_millis,
            "結合": binding_json(entry),
            "統治": governance_json(),
            "計測": all_unknown_metrics(reason),
            "短期履歴": Vec::<Value>::new(),
        }),
        evidence_source: "INTERNAL_STATE",
    }
}

fn binding_json(entry: &RuntimeRegistration) -> Value {
    let mut result = json!({
        "状態": entry.binding.status,
        "根拠": entry.binding.basis,
        "PID": entry.binding.pid,
        "PID作成時刻UnixMillis": entry.binding.creation_time_unix_millis,
        "登録時刻UnixMillis": entry.registered_at_millis,
        "登録監査ID": entry.registration_audit_id,
    });
    if let Some(reason) = entry.binding.reason {
        result["理由"] = Value::String(reason.to_string());
    }
    result
}

fn governance_json() -> Value {
    json!({
        "能力ID": "runtime.resource.observe",
        "権限ID": "permission.runtime.resource.observe",
        "承認状態": "not_required",
        "復旧ID": "recover-runtime-resource-binding",
    })
}

fn measured_metrics(
    sample: ProcessSample,
    uptime_millis: Option<u64>,
    cpu_percent: Option<f64>,
    stats: 実行系資源統計,
) -> Value {
    json!({
        "稼働時間Millis": uptime_millis.map(|value| measured(value, "LIVE_RUNTIME")).unwrap_or_else(|| unknown("LIVE_RUNTIME", "現在時刻とprocess作成時刻の差を安全に測定できません")),
        "CPU累積時間Millis": cpu_duration_metric(sample.total_cpu_100ns, "LIVE_RUNTIME"),
        "CPU利用率Percent": cpu_percent.and_then(|value| measured_decimal(value, "LIVE_RUNTIME")).unwrap_or_else(|| unknown("LIVE_RUNTIME", "初回観測、時間差不足、または安全な精度不足のためCPU変化率を測定できません")),
        "RAMWorkingSetBytes": measured(sample.working_set_bytes, "LIVE_RUNTIME"),
        "RAMPrivateBytes": measured(sample.private_bytes, "LIVE_RUNTIME"),
        "DiskIOBytes": unknown("INTERNAL_STATE", "Disk I/OはこのWindows collectorで分離測定していません"),
        "NetworkIOBytes": unknown("INTERNAL_STATE", "Network I/OはこのWindows collectorで分離測定していません"),
        "GPU利用率Percent": unknown("INTERNAL_STATE", "GPU利用率はこのWindows collectorで測定していません"),
        "VRAMBytes": unknown("INTERNAL_STATE", "VRAMはこのWindows collectorで測定していません"),
        "処理中要求数": measured(stats.処理中要求数, "INTERNAL_STATE"),
        "平均応答Millis": unknown("INTERNAL_STATE", "対話の応答時間をミリ秒精度で測定していません"),
        "失敗要求数": measured(stats.失敗要求数, "INTERNAL_STATE"),
    })
}

fn all_unknown_metrics(reason: &'static str) -> Value {
    json!({
        "稼働時間Millis": unknown("INTERNAL_STATE", reason),
        "CPU累積時間Millis": unknown("INTERNAL_STATE", reason),
        "CPU利用率Percent": unknown("INTERNAL_STATE", reason),
        "RAMWorkingSetBytes": unknown("INTERNAL_STATE", reason),
        "RAMPrivateBytes": unknown("INTERNAL_STATE", reason),
        "DiskIOBytes": unknown("INTERNAL_STATE", reason),
        "NetworkIOBytes": unknown("INTERNAL_STATE", reason),
        "GPU利用率Percent": unknown("INTERNAL_STATE", reason),
        "VRAMBytes": unknown("INTERNAL_STATE", reason),
        "処理中要求数": unknown("INTERNAL_STATE", reason),
        "平均応答Millis": unknown("INTERNAL_STATE", reason),
        "失敗要求数": unknown("INTERNAL_STATE", reason),
    })
}

fn error_rate_metric(stats: 実行系資源統計) -> Value {
    if stats.完了要求数 == 0 {
        return unknown("INTERNAL_STATE", "完了した対話記録がないためエラー率を測定できません");
    }
    if stats.失敗要求数 > stats.完了要求数 {
        return unknown("INTERNAL_STATE", "完了要求数と失敗要求数の統計が矛盾しています");
    }
    if stats.完了要求数 > MAX_EXACT_INTEGER || stats.失敗要求数 > MAX_EXACT_INTEGER {
        return unknown("INTERNAL_STATE", "エラー率を安全な精度で計算できません");
    }
    let percent = (stats.失敗要求数 as f64) * 100.0 / (stats.完了要求数 as f64);
    measured_decimal(percent, "INTERNAL_STATE")
        .unwrap_or_else(|| unknown("INTERNAL_STATE", "エラー率を数値へ安全に変換できません"))
}

fn measured(value: u64, source: &str) -> Value {
    json!({"状態": "measured", "値": value, "証拠種別": source})
}

fn measured_decimal(value: f64, source: &str) -> Option<Value> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    serde_json::Number::from_f64(value)
        .map(|number| json!({"状態": "measured", "値": Value::Number(number), "証拠種別": source}))
}

fn cpu_duration_metric(total_cpu_100ns: u64, source: &str) -> Value {
    let whole_millis = total_cpu_100ns / 10_000;
    if total_cpu_100ns % 10_000 == 0 {
        return measured(whole_millis, source);
    }
    if total_cpu_100ns > MAX_EXACT_INTEGER {
        return unknown(source, "CPU累積時間を安全な精度でミリ秒へ変換できません");
    }
    measured_decimal((total_cpu_100ns as f64) / 10_000.0, source)
        .unwrap_or_else(|| unknown(source, "CPU累積時間を数値へ安全に変換できません"))
}

fn cpu_percent_from_raw(
    current_cpu_100ns: u64,
    previous_cpu_100ns: u64,
    elapsed_100ns: u128,
    logical_processors: u64,
) -> Option<f64> {
    if current_cpu_100ns < previous_cpu_100ns
        || elapsed_100ns == 0
        || logical_processors == 0
        || current_cpu_100ns > MAX_EXACT_INTEGER
        || previous_cpu_100ns > MAX_EXACT_INTEGER
        || elapsed_100ns > u128::from(MAX_EXACT_INTEGER)
        || logical_processors > MAX_EXACT_INTEGER
    {
        return None;
    }
    let delta_cpu_100ns = current_cpu_100ns - previous_cpu_100ns;
    let percent = (delta_cpu_100ns as f64) * 100.0
        / (elapsed_100ns as f64)
        / (logical_processors as f64);
    if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
        return None;
    }
    Some(percent)
}

fn unknown(source: &str, reason: &str) -> Value {
    json!({"状態": "unknown", "値": Value::Null, "証拠種別": source, "理由": reason})
}

fn filetime_to_unix_millis(value: u64) -> Option<i64> {
    let unix_100ns = value.checked_sub(WINDOWS_UNIX_EPOCH_OFFSET_100NS)?;
    i64::try_from(unix_100ns / 10_000).ok()
}

#[cfg(windows)]
fn capture_process(pid: u32) -> Result<ProcessSample, ()> {
    use winsafe::{co, HPROCESS};

    let process = HPROCESS::OpenProcess(co::PROCESS::QUERY_LIMITED_INFORMATION, false, pid)
        .map_err(|_| ())?;
    let (creation, _exit, kernel, user) = process.GetProcessTimes().map_err(|_| ())?;
    let memory = process.GetProcessMemoryInfo().map_err(|_| ())?;
    Ok(ProcessSample {
        creation_time_100ns: u64::from(creation),
        total_cpu_100ns: u64::from(kernel).saturating_add(u64::from(user)),
        working_set_bytes: memory.WorkingSetSize as u64,
        private_bytes: memory.PrivateUsage as u64,
        sampled_at: Instant::now(),
    })
}

#[cfg(not(windows))]
fn capture_process(_pid: u32) -> Result<ProcessSample, ()> {
    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bound_registration() -> RuntimeRegistration {
        RuntimeRegistration {
            registered_at_millis: 1_700_000_000_000,
            registration_audit_id: "registration-audit-1".to_string(),
            binding: RuntimeBinding {
                status: "bound",
                basis: "loopback_tcp_listener_owner_pid",
                target: Some(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 10_001)),
                pid: Some(4242),
                creation_time_100ns: Some(WINDOWS_UNIX_EPOCH_OFFSET_100NS + 10_000),
                creation_time_unix_millis: Some(1),
                reason: None,
            },
            previous_sample: Some(CpuSample {
                total_cpu_100ns: 10,
                sampled_at: Instant::now(),
            }),
            history: VecDeque::from([json!({"観測時刻UnixMillis": 1})]),
        }
    }

    fn test_process_sample(creation_time_100ns: u64) -> ProcessSample {
        ProcessSample {
            creation_time_100ns,
            total_cpu_100ns: 10,
            working_set_bytes: 20,
            private_bytes: 30,
            sampled_at: Instant::now(),
        }
    }

    #[test]
    fn 返却sampleの後にendpoint所有者が変化した場合は観測を拒否する() {
        let expected_pid = 4242;
        let expected_creation = WINDOWS_UNIX_EPOCH_OFFSET_100NS + 10_000;
        let mut owners = vec![Ok(Some(expected_pid)), Ok(Some(expected_pid)), Ok(Some(9999))]
            .into_iter();
        let mut sampled = 0;
        let result = verify_current_binding_sample(
            expected_pid,
            expected_creation,
            || owners.next().expect("owner照合回数が不足"),
            |_| {
                sampled += 1;
                Ok(test_process_sample(expected_creation))
            },
        );
        assert!(matches!(result, Err(CurrentBindingError::Mismatch(_))));
        assert_eq!(sampled, 2, "返却予定sampleの後にも所有者を照合する");
    }

    #[test]
    fn endpoint消失は二回目のsampleを作らず観測を拒否する() {
        let expected_pid = 4242;
        let expected_creation = WINDOWS_UNIX_EPOCH_OFFSET_100NS + 10_000;
        let mut owners = vec![Ok(Some(expected_pid)), Ok(None)].into_iter();
        let mut sampled = 0;
        let result = verify_current_binding_sample(
            expected_pid,
            expected_creation,
            || owners.next().expect("owner照合回数が不足"),
            |_| {
                sampled += 1;
                Ok(test_process_sample(expected_creation))
            },
        );
        assert!(matches!(result, Err(CurrentBindingError::Mismatch(_))));
        assert_eq!(sampled, 1);
    }

    #[test]
    fn 登録中のsample後にendpoint所有者が変化した場合はbindingを保存しない() {
        let expected_pid = 4242;
        let expected_creation = WINDOWS_UNIX_EPOCH_OFFSET_100NS + 10_000;
        let mut owners = vec![
            Ok(Some(expected_pid)),
            Ok(Some(expected_pid)),
            Ok(Some(9999)),
        ]
        .into_iter();
        let mut sampled = 0;
        let result = verify_registration_binding(
            || owners.next().expect("owner照合回数が不足"),
            |_| {
                sampled += 1;
                Ok(test_process_sample(expected_creation))
            },
        );
        assert!(matches!(result, Err(RegistrationBindingError::Unbound(_))));
        assert_eq!(sampled, 2, "最後のowner照合より前にだけsampleを取る");
    }

    #[test]
    fn 登録中の二回目process読取不能はexitedとして返す() {
        let expected_pid = 4242;
        let expected_creation = WINDOWS_UNIX_EPOCH_OFFSET_100NS + 10_000;
        let mut owners = vec![
            Ok(Some(expected_pid)),
            Ok(Some(expected_pid)),
            Ok(Some(expected_pid)),
        ]
        .into_iter();
        let mut samples = vec![Ok(test_process_sample(expected_creation)), Err(())].into_iter();
        let result = verify_registration_binding(
            || owners.next().expect("owner照合回数が不足"),
            |_| samples.next().expect("process読取回数が不足"),
        );
        assert!(matches!(result, Err(RegistrationBindingError::Exited(_))));
    }

    #[test]
    fn bound結合は理由を露出せずPID再利用疑いで全資源履歴を破棄する() {
        let mut entry = bound_registration();
        assert!(binding_json(&entry).get("理由").is_none());

        invalidate_binding(
            &mut entry,
            "binding_mismatch",
            "PID再利用またはprocess生成時刻の不一致を検出しました",
        );
        let observation = unknown_observation("local", &entry, 1_700_000_000_100);
        assert_eq!(observation.evidence_source, "INTERNAL_STATE");
        assert_eq!(observation.body["結合"]["状態"], "binding_mismatch");
        assert!(observation.body["結合"]["PID"].is_null());
        assert!(observation.body["結合"]["PID作成時刻UnixMillis"].is_null());
        assert!(entry.binding.target.is_none());
        assert_eq!(observation.body["短期履歴"], json!([]));
        assert!(observation.body["計測"].as_object().unwrap().values().all(|metric| {
            metric["状態"] == "unknown" && metric["値"].is_null()
        }));
    }

    #[test]
    fn windows_filetimeはunix_millisへ安全に変換する() {
        assert_eq!(
            filetime_to_unix_millis(WINDOWS_UNIX_EPOCH_OFFSET_100NS + 12_340_000),
            Some(1_234),
        );
        assert_eq!(filetime_to_unix_millis(0), None);
    }

    #[test]
    fn cpuとエラー率は小数を切り捨てず安全な境界外をunknownにする() {
        let cpu_percent = cpu_percent_from_raw(2, 1, 100, 3).expect("小数CPU利用率を計測する");
        assert!((cpu_percent - (100.0 / 300.0)).abs() < f64::EPSILON);
        assert!(cpu_percent > 0.0 && cpu_percent < 1.0);

        let cpu_duration = cpu_duration_metric(12_500, "LIVE_RUNTIME");
        assert_eq!(cpu_duration["状態"], "measured");
        assert_eq!(cpu_duration["値"].as_f64(), Some(1.25));

        let third = error_rate_metric(実行系資源統計 {
            完了要求数: 3,
            失敗要求数: 1,
            ..Default::default()
        });
        let third_value = third["値"].as_f64().expect("小数エラー率を返す");
        assert!((third_value - (100.0 / 3.0)).abs() < f64::EPSILON);

        let inconsistent = error_rate_metric(実行系資源統計 {
            完了要求数: 1,
            失敗要求数: 2,
            ..Default::default()
        });
        assert_eq!(inconsistent["状態"], "unknown");
        assert!(inconsistent["値"].is_null());

        assert!(cpu_percent_from_raw(1, 2, 100, 1).is_none());
        assert!(cpu_percent_from_raw(2, 1, 0, 1).is_none());
        assert!(cpu_percent_from_raw(102, 1, 100, 1).is_none());
    }

    #[test]
    fn 平均応答は高精度時計なしではunknownに固定する() {
        let metrics = measured_metrics(
            test_process_sample(WINDOWS_UNIX_EPOCH_OFFSET_100NS + 10_000),
            Some(10),
            Some(1.0),
            実行系資源統計 {
                処理中要求数: 0,
                完了要求数: 1,
                失敗要求数: 0,
            },
        );
        assert_eq!(metrics["平均応答Millis"]["状態"], "unknown");
        assert_eq!(metrics["平均応答Millis"]["証拠種別"], "INTERNAL_STATE");
        assert!(metrics["平均応答Millis"]["値"].is_null());
    }
}
