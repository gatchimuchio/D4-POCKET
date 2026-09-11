//! build / release専用。署名鍵を読まず、公開鍵による検証だけを行う。
use crate::audit_hash::sha256_tagged;
use crate::broker::audit::{BrokerAuditEvent, BrokerAuditLog};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub format: String,
    pub version: u32,
    pub audit_chain_head: Option<String>,
    pub audit_log_sha256: String,
    pub audit_anchor_sha256: String,
    pub source_commit: String,
    pub installed_artifact_sha256: String,
    pub generated_at: u64,
    pub sequence: u64,
    pub previous_checkpoint_hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedHead {
    pub version: u32,
    pub sequence: u64,
    pub signed_checkpoint_hash: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trust {
    pub version: u32,
    pub algorithm: String,
    pub public_key_fingerprint: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Anchor {
    version: u32,
    event_count: usize,
    head_event_hash: Option<String>,
    anchor_hmac: String,
}

pub fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let f = std::fs::File::open(path).map_err(|_| "証拠fileを開けない")?;
    if !f.metadata().map_err(|_| "証拠metadataを読めない")?.is_file() {
        return Err("証拠は通常fileに限定する".into());
    }
    let mut bytes = Vec::new();
    f.take(limit + 1).read_to_end(&mut bytes).map_err(|_| "証拠を読めない")?;
    if bytes.len() as u64 > limit { return Err("証拠上限超過".into()); }
    Ok(bytes)
}

fn tagged(value: &str) -> bool {
    value.len() == 71 && value.starts_with("sha256:") && value[7..].bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

pub fn canonical(cp: &Checkpoint) -> Result<Vec<u8>, String> {
    if cp.format != "gui-shell-audit-checkpoint" || cp.version != 2 || cp.sequence == 0 || cp.generated_at == 0
        || cp.source_commit.len() != 40 || !cp.source_commit.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        || ![&cp.audit_log_sha256, &cp.audit_anchor_sha256, &cp.installed_artifact_sha256].iter().all(|s| tagged(s))
        || cp.audit_chain_head.as_ref().is_some_and(|s| !tagged(s))
        || cp.previous_checkpoint_hash.as_ref().is_some_and(|s| !tagged(s))
        || (cp.sequence == 1) != cp.previous_checkpoint_hash.is_none()
    { return Err("checkpoint構造・値が不正".into()); }
    serde_json::to_vec(cp).map_err(|_| "checkpoint正本化失敗".into())
}

pub fn parse(bytes: &[u8]) -> Result<Checkpoint, String> {
    let cp: Checkpoint = serde_json::from_slice(bytes).map_err(|_| "checkpoint構造不正")?;
    if canonical(&cp)? != bytes { return Err("canonical byte不一致".into()); }
    Ok(cp)
}

pub fn public_key(der: &[u8]) -> Result<&[u8], String> {
    // RFC8410 Ed25519 SPKI。別algorithm、付加parameter、後続byteを受理しない。
    const PREFIX: [u8; 12] = [0x30,0x2a,0x30,0x05,0x06,0x03,0x2b,0x65,0x70,0x03,0x21,0x00];
    if der.len() != 44 || der[..12] != PREFIX { return Err("Ed25519 SPKI公開鍵が必要".into()); }
    Ok(&der[12..])
}

pub fn signed_hash(bytes: &[u8], signature: &[u8]) -> String {
    sha256_tagged(&[bytes, signature].concat())
}

pub fn signature_check(bytes: &[u8], signature: &[u8], der: &[u8], trust: &Trust) -> Result<Checkpoint, String> {
    let key = public_key(der)?;
    if trust.version != 1 || trust.algorithm != "Ed25519" || trust.public_key_fingerprint.as_deref() != Some(&sha256_tagged(key)) {
        return Err("Repository固定公開鍵fingerprint不一致・未設定".into());
    }
    if signature.len() != 64 { return Err("署名長不正".into()); }
    UnparsedPublicKey::new(&ED25519, key).verify(bytes, signature).map_err(|_| "Ed25519署名不正")?;
    parse(bytes)
}

pub fn measure(store: &Path, artifact: &Path, source: &str, sequence: u64, previous: Option<String>, now: u64) -> Result<Checkpoint, String> {
    let log = read(&store.join("audit.jsonl"), 256 * 1024 * 1024)?;
    let anchor_bytes = read(&store.join("audit_anchor.json"), 64 * 1024)?;
    let text = std::str::from_utf8(&log).map_err(|_| "audit UTF-8不正")?;
    let events: Vec<BrokerAuditEvent> = text.lines().map(|line| serde_json::from_str(line).map_err(|_| "audit event不正".to_string())).collect::<Result<_, _>>()?;
    BrokerAuditLog::from_verified_events(events.clone())?;
    let anchor: Anchor = serde_json::from_slice(&anchor_bytes).map_err(|_| "anchor構造不正")?;
    let head = events.last().map(|e| e.event_hash.clone());
    if anchor.version != 1 || anchor.event_count != events.len() || anchor.head_event_hash != head || !tagged(&anchor.anchor_hmac) {
        return Err("anchorとaudit chain不一致".into());
    }
    let artifact_bytes = crate::installed_artifact::canonical(artifact)?;
    if read(&store.join("audit.jsonl"), 256 * 1024 * 1024)? != log || read(&store.join("audit_anchor.json"), 64 * 1024)? != anchor_bytes {
        return Err("採取中に監査fileが変化".into());
    }
    let cp = Checkpoint { format: "gui-shell-audit-checkpoint".into(), version: 2,
        audit_chain_head: head, audit_log_sha256: sha256_tagged(&log), audit_anchor_sha256: sha256_tagged(&anchor_bytes),
        source_commit: source.into(), installed_artifact_sha256: sha256_tagged(&artifact_bytes), generated_at: now, sequence,
        previous_checkpoint_hash: previous };
    canonical(&cp)?;
    Ok(cp)
}

pub fn verify(bytes: &[u8], sig: &[u8], der: &[u8], trust: &Trust, floor: &TrustedHead,
    previous: Option<(&[u8], &[u8])>, measured: &Checkpoint, now: u64) -> Result<TrustedHead, String> {
    let cp = signature_check(bytes, sig, der, trust)?;
    let hash = signed_hash(bytes, sig);
    if floor.version != 1 || (floor.sequence == 0) != floor.signed_checkpoint_hash.is_none()
        || floor.signed_checkpoint_hash.as_ref().is_some_and(|s| !tagged(s)) {
        return Err("信頼済み継続性記録不正".into());
    }
    if cp.sequence < floor.sequence || cp.sequence > floor.sequence.saturating_add(1) {
        return Err("sequence後退・欠番".into());
    }
    if cp.sequence == floor.sequence {
        if floor.signed_checkpoint_hash.as_ref() != Some(&hash) { return Err("同一sequenceの別checkpoint".into()); }
    } else if cp.previous_checkpoint_hash != floor.signed_checkpoint_hash {
        return Err("信頼済み前checkpoint hash不一致".into());
    }
    match previous {
        Some((b, s)) => {
            let prev = signature_check(b, s, der, trust)?;
            if prev.sequence.checked_add(1) != Some(cp.sequence) || cp.previous_checkpoint_hash.as_ref() != Some(&signed_hash(b, s)) || prev.generated_at > cp.generated_at {
                return Err("直前checkpoint連続性不正".into());
            }
        }
        None if cp.sequence != 1 => return Err("直前署名checkpointが必要".into()),
        None => {}
    }
    if cp.generated_at > now.saturating_add(60) || now.saturating_sub(cp.generated_at) > 86400 {
        return Err("checkpoint時刻の許容範囲外".into());
    }
    if cp.audit_chain_head != measured.audit_chain_head || cp.audit_log_sha256 != measured.audit_log_sha256
        || cp.audit_anchor_sha256 != measured.audit_anchor_sha256 || cp.source_commit != measured.source_commit
        || cp.installed_artifact_sha256 != measured.installed_artifact_sha256 {
        return Err("現在の監査・source・artifactと署名checkpoint不一致".into());
    }
    Ok(TrustedHead {version: 1, sequence: cp.sequence, signed_checkpoint_hash: Some(hash)})
}
