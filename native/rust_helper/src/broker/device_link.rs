//! 端末資格・所有関係の制御。UI、通信先metadata、保存値から権限を作らない。
#![allow(non_snake_case)]

use super::dialogue::識別子生成;
use crate::audit_hash::sha256_tagged;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::net::Ipv4Addr;
use subtle::ConstantTimeEq;

/// serdeの通常Value読取が上書きする重複keyも、内容を含む全階層で拒否する。
pub(crate) fn 要求読取(raw: &str) -> Result<端末要求, &'static str> {
    if raw.len() > 65536 {
        return Err("要求上限");
    }
    super::json_input::read_unique(raw).map_err(|_| "要求不正")
}

pub(crate) const 許可操作: &[&str] = &[
    "実行系列挙",
    "Agent一覧",
    "作業領域一覧",
    "実行系ライフサイクル状態",
    "実行系資源観測",
    "通知一覧",
    "全Runtime停止要求",
    "対話履歴閲覧状態",
    "対話履歴閲覧",
    "対話開始",
    "対話送信",
    "対話取得",
    "対話中止",
    "対話終了",
    "端末確認",
    "端末離脱",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct 端末要求 {
    pub 版: u32,
    pub HostID: String,
    pub 端末ID: String,
    pub 資格ID: String,
    pub 資格秘密: String,
    pub nonce: String,
    pub 発行時刻: i64,
    pub 操作: String,
    pub 内容: Value,
}

#[derive(Debug)]
struct 招待 {
    端末ID: String,
    秘密hash: String,
    Host: String,
    期限: i64,
}
#[derive(Debug)]
struct 結合 {
    端末ID: String,
    秘密hash: String,
    期限: i64,
    nonce: BTreeSet<String>,
    セッション: BTreeSet<String>,
    要求: BTreeSet<String>,
}

#[derive(Debug)]
pub(crate) struct 端末制御 {
    pub HostID: String,
    証明書hash: String,
    port: u16,
    招待: BTreeMap<String, 招待>,
    結合: BTreeMap<String, 結合>,
    pub 停止: bool,
}

pub(crate) fn hex形状(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn 接続先検査(s: &str) -> bool {
    s.parse::<Ipv4Addr>()
        .is_ok_and(|ip| (ip.is_private() || ip.is_loopback()) && !ip.is_broadcast())
}
fn 秘密生成() -> Result<String, &'static str> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|_| "乱数生成失敗")?;
    Ok(hex::encode(bytes))
}
fn 一致(a: &str, b: &str) -> bool {
    bool::from(a.as_bytes().ct_eq(b.as_bytes()))
}
fn field<'a>(p: &'a Value, key: &str) -> Result<&'a str, &'static str> {
    p.get(key).and_then(Value::as_str).ok_or("要求不正")
}
fn fields(p: &Value, keys: &[&str]) -> bool {
    p.as_object()
        .is_some_and(|m| m.len() == keys.len() && keys.iter().all(|k| m.contains_key(*k)))
}

impl 端末制御 {
    pub fn 新規(証明書hash: String, port: u16) -> Result<Self, &'static str> {
        if !hex形状(&証明書hash, 64) || port == 0 {
            return Err("接続先不正");
        }
        Ok(Self {
            HostID: 識別子生成().map_err(|_| "乱数生成失敗")?,
            証明書hash,
            port,
            招待: BTreeMap::new(),
            結合: BTreeMap::new(),
            停止: false,
        })
    }

    pub fn 制御(
        &mut self,
        操作: &str,
        p: &Value,
        now: i64,
    ) -> Result<(Value, Vec<String>), &'static str> {
        if self.停止 {
            return Err("監査修復が必要");
        }
        match 操作 {
            "端末招待" if fields(p, &["端末ID", "接続先Host"]) => {
                let device = field(p, "端末ID")?;
                let host = field(p, "接続先Host")?;
                if !hex形状(device, 32) || !接続先検査(host) {
                    return Err("接続先または端末ID不正");
                }
                if self.招待.len() >= 32 || self.結合.values().any(|v| v.端末ID == device) {
                    return Err("結合上限または既存結合");
                }
                let id = 識別子生成().map_err(|_| "乱数生成失敗")?;
                let secret = 秘密生成()?;
                let expiry = now.checked_add(300).ok_or("時刻不正")?;
                self.招待.insert(
                    id.clone(),
                    招待 {
                        端末ID: device.into(),
                        秘密hash: sha256_tagged(secret.as_bytes()),
                        Host: host.into(),
                        期限: expiry,
                    },
                );
                Ok((
                    json!({"版":1,"HostID":self.HostID,"接続先Host":host,"port":self.port,"証明書hash":self.証明書hash,
                    "端末ID":device,"有効期限":expiry,"招待ID":id,"招待秘密":secret}),
                    vec![],
                ))
            }
            "端末一覧" if fields(p, &[]) => Ok((
                json!({"HostID":self.HostID,"証明書hash":self.証明書hash,"port":self.port,
                "招待":self.招待.iter().map(|(id,v)|json!({"招待ID":id,"端末ID":v.端末ID,"有効期限":v.期限})).collect::<Vec<_>>(),
                "結合":self.結合.iter().map(|(id,v)|json!({"結合ID":id,"端末ID":v.端末ID,"有効期限":v.期限})).collect::<Vec<_>>()}),
                vec![],
            )),
            "端末招待取消" if fields(p, &["招待ID"]) => {
                self.招待.remove(field(p, "招待ID")?).ok_or("招待不在")?;
                Ok((json!({"状態":"取消"}), vec![]))
            }
            "端末失効" if fields(p, &["結合ID"]) => {
                let sessions = self.失効(field(p, "結合ID")?)?;
                Ok((json!({"状態":"失効"}), sessions))
            }
            _ => Err("要求不正"),
        }
    }

    pub fn 期限処理(&mut self, now: i64) -> (Vec<String>, Vec<String>) {
        let mut expired: Vec<_> = self
            .招待
            .iter()
            .filter(|(_, v)| now >= v.期限)
            .map(|(id, _)| id.clone())
            .collect();
        self.招待.retain(|_, v| now < v.期限);
        let ids: Vec<_> = self
            .結合
            .iter()
            .filter(|(_, v)| now >= v.期限)
            .map(|(id, _)| id.clone())
            .collect();
        expired.extend(ids.iter().cloned());
        (
            expired,
            ids.iter()
                .flat_map(|id| self.失効(id).unwrap_or_default())
                .collect(),
        )
    }

    pub fn 結合する(&mut self, r: &端末要求, now: i64) -> Result<Value, &'static str> {
        self.共通検査(r, now)?;
        if r.操作 != "端末結合" || !fields(&r.内容, &[]) {
            return Err("要求不正");
        }
        let invitation = self.招待.get(&r.資格ID).ok_or("資格拒否")?;
        if invitation.端末ID != r.端末ID
            || now >= invitation.期限
            || !一致(&invitation.秘密hash, &sha256_tagged(r.資格秘密.as_bytes()))
        {
            return Err("資格拒否");
        }
        if self.結合.len() >= 32 || self.結合.values().any(|v| v.端末ID == r.端末ID) {
            return Err("結合上限または既存結合");
        }
        let id = 識別子生成().map_err(|_| "乱数生成失敗")?;
        let secret = 秘密生成()?;
        let expiry = now.checked_add(8 * 60 * 60).ok_or("時刻不正")?;
        let host = invitation.Host.clone();
        self.招待.remove(&r.資格ID);
        self.結合.insert(
            id.clone(),
            結合 {
                端末ID: r.端末ID.clone(),
                秘密hash: sha256_tagged(secret.as_bytes()),
                期限: expiry,
                nonce: BTreeSet::from([r.nonce.clone()]),
                セッション: BTreeSet::new(),
                要求: BTreeSet::new(),
            },
        );
        Ok(
            json!({"版":1,"HostID":self.HostID,"接続先Host":host,"port":self.port,"証明書hash":self.証明書hash,
            "端末ID":r.端末ID,"有効期限":expiry,"結合ID":id,"端末秘密":secret}),
        )
    }

    fn 共通検査(&self, r: &端末要求, now: i64) -> Result<(), &'static str> {
        if self.停止 {
            return Err("監査修復が必要");
        }
        if r.版 != 1
            || r.HostID != self.HostID
            || !hex形状(&r.端末ID, 32)
            || !hex形状(&r.資格ID, 32)
            || !hex形状(&r.資格秘密, 64)
            || !hex形状(&r.nonce, 32)
            || r.発行時刻 <= 0
            || now.abs_diff(r.発行時刻) > 60
        {
            return Err("資格または時刻拒否");
        }
        Ok(())
    }

    pub fn 認証する(&mut self, r: &端末要求, now: i64) -> Result<(), &'static str> {
        self.共通検査(r, now)?;
        if !許可操作.contains(&r.操作.as_str()) {
            return Err("端末操作拒否");
        }
        let pair = self.結合.get_mut(&r.資格ID).ok_or("資格拒否")?;
        if pair.端末ID != r.端末ID
            || now >= pair.期限
            || !一致(&pair.秘密hash, &sha256_tagged(r.資格秘密.as_bytes()))
        {
            return Err("資格拒否");
        }
        if pair.nonce.len() >= 4096 || !pair.nonce.insert(r.nonce.clone()) {
            return Err("nonce再使用または上限");
        }
        match r.操作.as_str() {
            "実行系列挙" | "Agent一覧" | "作業領域一覧" | "端末確認" | "端末離脱"
                if !fields(&r.内容, &[]) =>
            {
                return Err("要求不正")
            }
            "対話送信" | "対話終了" => {
                if !pair
                    .セッション
                    .contains(field(&r.内容, "対話セッションID")?)
                {
                    return Err("対話所有者不一致");
                }
            }
            "対話取得" | "対話中止" => {
                if !pair.要求.contains(field(&r.内容, "要求ID")?) {
                    return Err("要求所有者不一致");
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn 所有記録(
        &mut self,
        id: &str,
        operation: &str,
        body: &Value,
    ) -> Result<(), &'static str> {
        let pair = self.結合.get_mut(id).ok_or("資格拒否")?;
        match operation {
            "対話開始" => {
                pair.セッション
                    .insert(field(body, "対話セッションID")?.into());
            }
            "対話送信" => {
                pair.要求.insert(field(body, "要求ID")?.into());
            }
            "対話終了" => {
                pair.セッション.remove(field(body, "対話セッションID")?);
            }
            _ => {}
        }
        Ok(())
    }
    pub fn 失効(&mut self, id: &str) -> Result<Vec<String>, &'static str> {
        Ok(self
            .結合
            .remove(id)
            .ok_or("結合不在")?
            .セッション
            .into_iter()
            .collect())
    }
    pub fn 全停止(&mut self) -> Vec<String> {
        self.停止 = true;
        self.招待.clear();
        std::mem::take(&mut self.結合)
            .into_values()
            .flat_map(|v| v.セッション)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn 準備() -> (端末制御, Value) {
        let mut c = 端末制御::新規("b".repeat(64), 7443).unwrap();
        let (i, _) = c
            .制御(
                "端末招待",
                &json!({"端末ID":"c".repeat(32),"接続先Host":"127.0.0.1"}),
                1000,
            )
            .unwrap();
        (c, i)
    }
    fn 要求(i: &Value, op: &str, now: i64) -> 端末要求 {
        要求読取(&json!({"版":1,"HostID":i["HostID"],"端末ID":i["端末ID"],"資格ID":i.get("結合ID").unwrap_or(&i["招待ID"]),
            "資格秘密":i.get("端末秘密").unwrap_or(&i["招待秘密"]),"nonce":識別子生成().unwrap(),"発行時刻":now,"操作":op,"内容":{}}).to_string()).unwrap()
    }
    #[test]
    fn 一回結合と再接続とnonce再使用を分離する() {
        let (mut c, i) = 準備();
        let r = 要求(&i, "端末結合", 1001);
        let credential = c.結合する(&r, 1001).unwrap();
        assert!(c.結合する(&r, 1001).is_err());
        let r = 要求(&credential, "端末確認", 1002);
        assert!(c.認証する(&r, 1002).is_ok());
        assert!(c.認証する(&r, 1002).is_err());
        assert!(c
            .認証する(&要求(&credential, "端末確認", 1003), 1003)
            .is_ok());
    }
    #[test]
    fn 招待取消と期限境界を拒否する() {
        let (mut c, i) = 準備();
        assert!(c.結合する(&要求(&i, "端末結合", 1300), 1300).is_err());
        c.制御("端末招待取消", &json!({"招待ID":i["招待ID"]}), 1001)
            .unwrap();
        assert!(c.結合する(&要求(&i, "端末結合", 1001), 1001).is_err());
        let (mut c, i) = 準備();
        let credential = c.結合する(&要求(&i, "端末結合", 1001), 1001).unwrap();
        assert!(c
            .認証する(&要求(&credential, "端末確認", 29801), 29801)
            .is_err());
        let (expired, _) = c.期限処理(29801);
        assert_eq!(expired.len(), 1);
    }
    #[test]
    fn 不正資格とHostと端末と時刻と昇格を拒否する() {
        let (mut c, i) = 準備();
        let credential = c.結合する(&要求(&i, "端末結合", 1001), 1001).unwrap();
        let mut r = 要求(&credential, "端末確認", 1002);
        r.資格秘密 = "0".repeat(64);
        assert!(c.認証する(&r, 1002).is_err());
        let mut r = 要求(&credential, "端末確認", 1002);
        r.HostID = "0".repeat(32);
        assert!(c.認証する(&r, 1002).is_err());
        let mut r = 要求(&credential, "端末確認", 1002);
        r.端末ID = "0".repeat(32);
        assert!(c.認証する(&r, 1002).is_err());
        assert!(c
            .認証する(&要求(&credential, "端末確認", 1002), 1063)
            .is_err());
        for op in ["対話承認", "shutdown", "端末招待", "command_envelope"] {
            assert!(c.認証する(&要求(&credential, op, 1002), 1002).is_err());
        }
    }
    #[test]
    fn 所有関係と失効を検査する() {
        let (mut c, i) = 準備();
        let credential = c.結合する(&要求(&i, "端末結合", 1001), 1001).unwrap();
        let id = credential["結合ID"].as_str().unwrap();
        c.所有記録(id, "対話開始", &json!({"対話セッションID":"a".repeat(32)}))
            .unwrap();
        c.所有記録(id, "対話送信", &json!({"要求ID":"b".repeat(32)}))
            .unwrap();
        let mut r = 要求(&credential, "対話取得", 1002);
        r.内容 = json!({"要求ID":"b".repeat(32)});
        assert!(c.認証する(&r, 1002).is_ok());
        r.nonce = 識別子生成().unwrap();
        r.内容 = json!({"要求ID":"f".repeat(32)});
        assert!(c.認証する(&r, 1002).is_err());
        let sessions = c.失効(id).unwrap();
        assert_eq!(sessions, vec!["a".repeat(32)]);
        assert!(c
            .認証する(&要求(&credential, "端末確認", 1003), 1003)
            .is_err());
    }
    #[test]
    fn 重複fieldと未知fieldを全階層で拒否する() {
        let (_, i) = 準備();
        let r = 要求(&i, "端末結合", 1001);
        let raw=json!({"版":1,"HostID":r.HostID,"端末ID":r.端末ID,"資格ID":r.資格ID,"資格秘密":r.資格秘密,"nonce":r.nonce,"発行時刻":r.発行時刻,"操作":r.操作,"内容":{}}).to_string();
        assert!(要求読取(&raw.replacen("{", "{\"版\":1,", 1)).is_err());
        assert!(要求読取(
            &raw.replace("\"内容\":{}", "\"内容\":{\"入力\":\"a\",\"入力\":\"b\"}")
        )
        .is_err());
        assert!(要求読取(&raw.replacen("{", "{\"owner\":true,", 1)).is_err());
    }
    #[test]
    fn 全停止は資格を復活させず接続先範囲を固定する() {
        let (mut c, i) = 準備();
        let credential = c.結合する(&要求(&i, "端末結合", 1001), 1001).unwrap();
        c.全停止();
        assert!(c
            .認証する(&要求(&credential, "端末確認", 1002), 1002)
            .is_err());
        assert!(c
            .制御(
                "端末招待",
                &json!({"端末ID":"c".repeat(32),"接続先Host":"127.0.0.1"}),
                1002
            )
            .is_err());
        for host in ["0.0.0.0", "8.8.8.8", "localhost", "::1", "192.168.0.256"] {
            assert!(!接続先検査(host));
        }
    }
}
