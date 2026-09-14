//! 対話の要求・承認・結果を所有する。実行系固有の通信やUIは所有しない。
#![allow(non_snake_case)]

use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

use crate::audit_hash::sha256_tagged;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 対話失敗 {
    要求不正,
    実行系不在,
    権限拒否,
    セッション不一致,
    通信失敗,
    期限超過,
    応答不正,
    監査失敗,
    取消,
    隔離済み,
}
impl 対話失敗 {
    pub fn 分類(self) -> &'static str {
        match self {
            Self::要求不正 => "要求不正",
            Self::実行系不在 => "実行系不在",
            Self::権限拒否 => "権限拒否",
            Self::セッション不一致 => "セッション不一致",
            Self::通信失敗 => "通信失敗",
            Self::期限超過 => "期限超過",
            Self::応答不正 => "応答不正",
            Self::監査失敗 => "監査失敗",
            Self::取消 => "取消",
            Self::隔離済み => "実行系隔離済み",
        }
    }
    pub fn 復旧(self) -> &'static str {
        match self {
            Self::要求不正 => "入力修正",
            Self::実行系不在 => "実行系再確認",
            Self::権限拒否 => "権限再確認",
            Self::監査失敗 => "監査修復",
            Self::セッション不一致 | Self::取消 | Self::期限超過 => "新規セッション",
            Self::通信失敗 | Self::応答不正 => "接続再確認",
            Self::隔離済み => "RecoveryActionによる隔離判断を確認",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct 対話要求 {
    pub 要求ID: String,
    pub 実行系ID: String,
    pub 対話セッションID: String,
    pub 入力: String,
}

pub struct 実行結果 {
    pub 対話セッションID: String,
    pub 本文: String,
    pub 参照: Vec<String>,
    pub 能力: Vec<String>,
    pub 経路: String,
    pub 追跡ID: String,
    pub 追跡hash: String,
    pub 保留: bool,
    pub 生応答: Vec<u8>,
}

pub trait 実行系Adapter: Send + Sync {
    fn 接続対象(&self) -> String;
    /// OS資源観測のためにBrokerだけが読むloopback接続先。権限や操作対象を生成しない。
    fn 観測対象(&self) -> Option<SocketAddr> {
        None
    }
    fn 応答(
        &self,
        要求: &対話要求,
        取消: &AtomicBool,
        期限: Instant,
        生受信: &mut Vec<Vec<u8>>,
    ) -> Result<実行結果, 対話失敗>;
}

/// Broker内の対話記録から得る統計。OS測定値ではなく、現在processの権限や健全性を示さない。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct 実行系資源統計 {
    pub 処理中要求数: u64,
    pub 完了要求数: u64,
    pub 失敗要求数: u64,
}

/// IPCの資源観測契約と起動制御面で共用する実行系IDの境界。
/// ASCII以外、PIDや接続先を埋め込む区切り文字、空白、制御文字は許可しない。
pub(crate) fn 実行系ID妥当(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 128
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(byte))
}

struct 受信結果 {
    結果: Result<実行結果, 対話失敗>,
    生受信: Vec<Vec<u8>>,
    /// Adapter呼出しが返った直後にworkerが採取する単調時刻。
    /// poll時刻を応答時間に混ぜない。
    応答完了: Option<Instant>,
}

#[derive(Serialize)]
struct セッション {
    対話セッションID: String,
    実行系ID: String,
    状態: String,
}

/// 対話開始を監査へ結び付ける正規の安全な射影。
/// Session作成と後続のC5復元検証が同じ表現を使うことで、監査IDの隣接関係を
/// 推測せずにSessionと実行系を検証できる。
fn 対話開始監査射影(対話セッションID: &str, 実行系ID: &str) -> Value {
    json!({
        "対話セッションID": 対話セッションID,
        "実行系ID": 実行系ID,
        "状態": "利用中",
    })
}

/// C5の復元検証が共有する対話開始監査hash。private入力や接続先を含まない。
pub(crate) fn 対話開始監査hash(対話セッションID: &str, 実行系ID: &str) -> String {
    let body = 対話開始監査射影(対話セッションID, 実行系ID);
    sha256_tagged(body.to_string().as_bytes())
}
struct 作業 {
    要求: 対話要求,
    要求hash: String,
    作成時刻: i64,
    開始時刻: Option<i64>,
    終了時刻: Option<i64>,
    保存済み記録hash: Option<String>,
    保存済み結果証跡: bool,
    作成監査ID: String,
    開始監査ID: Option<String>,
    終了監査ID: Option<String>,
    状態: &'static str,
    表示範囲: String,
    /// C5評価が既存の対話経路を使うための内部区別。
    /// 通常の`対話取得`からはownerを含めて結果を返さない。
    評価隔離: bool,
    取消: Arc<AtomicBool>,
    受信: Option<mpsc::Receiver<受信結果>>,
    生受信: Vec<Vec<u8>>,
    結果: Option<Result<実行結果, 対話失敗>>,
    実行期限: Option<Instant>,
    /// C5の遅延評価だけが使う単調時計。監査の壁時計やC3の資源観測値を代用しない。
    単調開始: Option<Instant>,
    単調応答Millis: Option<u64>,
}

/// 評価ラボが既存の対話統治経路から受け取る内部射影。
/// 生応答、入力、Adapter接続先は含めない。`結果` は既存の表示範囲射影である。
#[derive(Clone)]
pub(crate) struct 評価対話進捗 {
    pub 要求ID: String,
    pub 実行系ID: String,
    pub 対話セッションID: String,
    pub 状態: String,
    pub 結果: Option<Value>,
    pub 実行記録: Value,
    pub 単調応答Millis: Option<u64>,
}

#[derive(Default)]
pub struct 対話制御 {
    実行系: BTreeMap<String, Arc<dyn 実行系Adapter>>,
    セッション: BTreeMap<String, セッション>,
    作業: BTreeMap<String, 作業>,
    失効セッション: BTreeSet<String>,
}
impl std::fmt::Debug for 対話制御 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("対話制御")
            .field("実行系数", &self.実行系.len())
            .field("要求数", &self.作業.len())
            .finish()
    }
}
impl Drop for 対話制御 {
    fn drop(&mut self) {
        for 作業 in self.作業.values() {
            作業.取消.store(true, Ordering::SeqCst);
        }
    }
}

type 監査器<'a> = dyn FnMut(&str, &str, &str) -> Result<String, 対話失敗> + 'a;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 実行系指定 {
    実行系ID: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 送信指定 {
    対話セッションID: String,
    入力: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 要求指定 {
    要求ID: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 終了指定 {
    対話セッションID: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct 承認指定 {
    要求ID: String,
    要求hash: String,
    表示範囲: String,
}

fn 読取<T: serde::de::DeserializeOwned>(値: &Value) -> Result<T, 対話失敗> {
    serde_json::from_value(値.clone()).map_err(|_| 対話失敗::要求不正)
}
fn 空入力(値: &Value) -> Result<(), 対話失敗> {
    if 値.as_object().is_some_and(|v| v.is_empty()) {
        Ok(())
    } else {
        Err(対話失敗::要求不正)
    }
}
pub fn 識別子生成() -> Result<String, 対話失敗> {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|_| 対話失敗::要求不正)?;
    Ok(hex::encode(bytes))
}
pub(crate) fn 要求hash(要求: &対話要求) -> String {
    let 値 = serde_json::to_value(要求).expect("対話要求のJSON変換");
    sha256_tagged(
        serde_json::to_string(&値)
            .expect("対話要求の正本化")
            .as_bytes(),
    )
}

impl 対話制御 {
    /// owner保存制御だけが消費する。本文を通常IPCへ返すAPIではない。
    /// 評価隔離済み対話は評価専用ProtectedStoreから外へ移さないため拒否する。
    #[cfg(any(windows, test))]
    pub(crate) fn 保存対象(&self, payload: &Value) -> Result<Value, 対話失敗> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct 指定 {
            要求ID: String,
            要求hash: String,
        }
        let p: 指定 = 読取(payload)?;
        let work = self.作業.get(&p.要求ID).ok_or(対話失敗::要求不正)?;
        if work.評価隔離 || p.要求hash != work.要求hash || work.表示範囲 != "full" {
            return Err(対話失敗::権限拒否);
        }
        if work.状態 != "完了"
            || !work.保存済み結果証跡
            || work.保存済み記録hash.is_none()
            || work.終了監査ID.is_none()
            || !matches!(work.結果, Some(Ok(_)))
        {
            return Err(対話失敗::要求不正);
        }
        Ok(json!({"版":1,"要求":work.要求,"要求hash":work.要求hash,
            "結果":表示射影(work, work.結果.as_ref().ok_or(対話失敗::要求不正)?),
            "実行記録":実行記録(work),"結果証跡":結果証跡(work).ok_or(対話失敗::要求不正)?}))
    }

    pub(crate) fn 登録済み(&self, id: &str) -> bool {
        self.実行系.contains_key(id)
    }

    /// C5が一括の承認待ち対話を作る前に、既存対話を追い出さずに確認する上限。
    /// この値は権限・Approval・送信許可を生成しない。
    pub(crate) fn 評価要求可能数(&self) -> usize {
        128usize
            .saturating_sub(self.作業.len())
            .min(64usize.saturating_sub(self.セッション.len()))
    }

    /// C4のterminal隔離後、評価対話のworker受信側とsessionの双方が実際に
    /// 解放されたことだけをC5が確認する。外部実行の停止やC4の成功は示さない。
    pub(crate) fn 評価対話回収済み(&self, 要求ID: &str, 対話セッションID: &str) -> bool {
        !self.作業.contains_key(要求ID) && !self.セッション.contains_key(対話セッションID)
    }

    /// C5評価だけが、送信済みでまだowner承認されていない対話を隔離する。
    /// 隔離後も既存のowner対話承認は個別に行うが、通常の`対話取得`へは公開しない。
    pub(crate) fn 評価隔離(&mut self, 要求ID: &str) -> Result<(), 対話失敗> {
        let work = self.作業.get_mut(要求ID).ok_or(対話失敗::要求不正)?;
        if work.評価隔離 || work.状態 != "承認待ち" || work.受信.is_some() || work.結果.is_some()
        {
            return Err(対話失敗::要求不正);
        }
        work.評価隔離 = true;
        Ok(())
    }

    /// C5の評価結果収集用。既存の対話状態遷移・監査保存を進めてから、
    /// Content Exposure Boundaryを適用済みの結果だけを返す。
    /// 評価器や呼出し元に生応答・入力・接続先を渡さない。
    pub(crate) fn 評価進捗(
        &mut self,
        要求ID: &str,
        現在: i64,
        監査: &mut 監査器<'_>,
    ) -> Result<評価対話進捗, 対話失敗> {
        self.進捗反映(現在, 監査)?;
        let work = self.作業.get(要求ID).ok_or(対話失敗::要求不正)?;
        if !work.評価隔離 {
            return Err(対話失敗::要求不正);
        }
        if work.状態 == "監査失敗" {
            return Err(対話失敗::監査失敗);
        }
        Ok(評価対話進捗 {
            要求ID: work.要求.要求ID.clone(),
            実行系ID: work.要求.実行系ID.clone(),
            対話セッションID: work.要求.対話セッションID.clone(),
            状態: work.状態.to_string(),
            結果: work.結果.as_ref().map(|result| 表示射影(work, result)),
            実行記録: 実行記録(work),
            単調応答Millis: work.単調応答Millis,
        })
    }

    /// C5の非隣接監査回帰だけが、workerから到着済みの結果を進捗反映前の同じ
    /// receiverへ戻すための試験用同期点。production binaryには含めない。
    #[cfg(test)]
    pub(crate) fn 試験用受信済みを進捗前に再投入(
        &mut self,
        要求ID: &str,
        deadline: Instant,
    ) -> Result<(), 対話失敗> {
        loop {
            let received = {
                let work = self.作業.get(要求ID).ok_or(対話失敗::要求不正)?;
                let receiver = work.受信.as_ref().ok_or(対話失敗::要求不正)?;
                match receiver.try_recv() {
                    Ok(value) => Some(value),
                    Err(mpsc::TryRecvError::Empty) => None,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return Err(対話失敗::通信失敗)
                    }
                }
            };
            if let Some(value) = received {
                let (sender, receiver) = mpsc::sync_channel(1);
                sender.send(value).map_err(|_| 対話失敗::通信失敗)?;
                self.作業
                    .get_mut(要求ID)
                    .ok_or(対話失敗::要求不正)?
                    .受信 = Some(receiver);
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(対話失敗::通信失敗);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// これはin-memory対話記録の射影であり、実行系processからのtelemetryではない。
    pub(crate) fn 資源統計(&self, 実行系ID: &str) -> 実行系資源統計 {
        let mut stats = 実行系資源統計::default();
        for work in self
            .作業
            .values()
            .filter(|work| work.要求.実行系ID == 実行系ID)
        {
            if work.状態 == "実行中" {
                stats.処理中要求数 = stats.処理中要求数.saturating_add(1);
            }
            let 完了 = matches!(work.結果, Some(_))
                && !matches!(work.状態, "承認待ち" | "実行中" | "監査失敗");
            if !完了 {
                continue;
            }
            stats.完了要求数 = stats.完了要求数.saturating_add(1);
            if matches!(work.結果, Some(Err(error)) if error != 対話失敗::取消) {
                stats.失敗要求数 = stats.失敗要求数.saturating_add(1);
            }
        }
        // 開始時刻・終了時刻は監査用の秒精度であり、ミリ秒応答時間の実測値ではない。
        // 高精度の単調時計を記録するまで、この統計に平均応答時間を持たせない。
        stats
    }

    /// 資格失効は監査障害時も採用停止を優先する。外部計算の停止は保証しない。
    pub(crate) fn 資格隔離(&mut self, sessions: &[String]) {
        self.失効セッション.extend(sessions.iter().cloned());
        for work in self.作業.values_mut() {
            if sessions.contains(&work.要求.対話セッションID) {
                work.取消.store(true, Ordering::SeqCst);
                work.状態 = "中止";
                work.結果 = Some(Err(対話失敗::取消));
                work.単調応答Millis = None;
            }
        }
        for id in sessions {
            if let Some(session) = self.セッション.get_mut(id) {
                session.状態 = "中止後隔離".into();
            }
        }
        self.失効資源解放();
    }

    fn 失効資源解放(&mut self) {
        let finished: Vec<_> = self
            .失効セッション
            .iter()
            .filter(|id| {
                !self
                    .作業
                    .values()
                    .any(|w| &w.要求.対話セッションID == *id && w.受信.is_some())
            })
            .cloned()
            .collect();
        for id in finished {
            self.作業.retain(|_, w| w.要求.対話セッションID != id);
            self.セッション.remove(&id);
            self.失効セッション.remove(&id);
        }
    }

    /// 起動制御面だけが登録する。登録は通信や送信許可を発生させない。
    pub fn 登録(
        &mut self,
        ID: &str,
        adapter: Arc<dyn 実行系Adapter>,
    ) -> Result<(), 対話失敗> {
        if !実行系ID妥当(ID) || self.実行系.contains_key(ID) {
            return Err(対話失敗::要求不正);
        }
        self.実行系.insert(ID.to_owned(), adapter);
        Ok(())
    }

    /// 起動制御面が、直後の登録監査に失敗した新規登録だけを取り消す。
    /// 対話sessionや作業を持つ実行系の管理操作には使わない。
    pub(crate) fn 直後登録取消(&mut self, ID: &str) -> bool {
        self.実行系.remove(ID).is_some()
    }

    /// terminal lifecycle隔離の後に、同じ実行系へ通常対話を再接続・再実行させない。
    /// 既存sessionと進行中要求も資格隔離と同じfail-closed処理で停止する。
    pub(crate) fn 実行系隔離(&mut self, 実行系ID: &str) {
        let sessions = self
            .セッション
            .iter()
            .filter_map(|(session_id, session)| {
                (session.実行系ID == 実行系ID).then(|| session_id.clone())
            })
            .collect::<Vec<_>>();
        self.資格隔離(&sessions);
        self.実行系.remove(実行系ID);
    }

    pub fn 操作(
        &mut self,
        操作: &str,
        値: &Value,
        owner: bool,
        現在: i64,
        監査: &mut 監査器<'_>,
    ) -> Result<Value, 対話失敗> {
        if matches!(操作, "対話承認" | "対話承認待ち") && !owner {
            return Err(対話失敗::権限拒否);
        }
        self.進捗反映(現在, 監査)?;
        let result = match 操作 {
            "実行系列挙" => {
                空入力(値)?;
                Ok(json!({"実行系": self.実行系.keys().collect::<Vec<_>>()}))
            }
            "対話開始" => {
                let 指定: 実行系指定 = 読取(値)?;
                if !self.実行系.contains_key(&指定.実行系ID) {
                    return Err(対話失敗::実行系不在);
                }
                if self.セッション.len() >= 64 {
                    return Err(対話失敗::要求不正);
                }
                let ID = 識別子生成()?;
                let session = セッション {
                    対話セッションID: ID.clone(),
                    実行系ID: 指定.実行系ID,
                    状態: "利用中".into(),
                };
                let body = 対話開始監査射影(&session.対話セッションID, &session.実行系ID);
                監査(
                    "対話開始",
                    &ID,
                    &対話開始監査hash(&session.対話セッションID, &session.実行系ID),
                )?;
                self.セッション.insert(ID, session);
                Ok(body)
            }
            "対話送信" => {
                let 指定: 送信指定 = 読取(値)?;
                if 指定.入力.trim().is_empty()
                    || 指定.入力.chars().count() > 4096
                    || self.作業.len() >= 128
                {
                    return Err(対話失敗::要求不正);
                }
                let session = self
                    .セッション
                    .get(&指定.対話セッションID)
                    .ok_or(対話失敗::セッション不一致)?;
                if session.状態 != "利用中"
                    || self.作業.values().any(|v| {
                        v.要求.対話セッションID == 指定.対話セッションID
                            && matches!(v.状態, "承認待ち" | "実行中")
                    })
                {
                    return Err(対話失敗::セッション不一致);
                }
                let 要求 = 対話要求 {
                    要求ID: 識別子生成()?,
                    実行系ID: session.実行系ID.clone(),
                    対話セッションID: 指定.対話セッションID,
                    入力: 指定.入力,
                };
                let hash = 要求hash(&要求);
                let 作成監査ID = 監査("対話承認待ち作成", &要求.要求ID, &hash)?;
                let body = json!({"要求ID": 要求.要求ID, "要求hash": hash, "状態": "承認待ち", "期限": 現在 + 300});
                self.作業.insert(
                    要求.要求ID.clone(),
                    作業 {
                        要求,
                        要求hash: hash,
                        作成時刻: 現在,
                        開始時刻: None,
                        終了時刻: None,
                        保存済み記録hash: None,
                        保存済み結果証跡: false,
                        作成監査ID,
                        開始監査ID: None,
                        終了監査ID: None,
                        状態: "承認待ち",
                        表示範囲: "none".into(),
                        評価隔離: false,
                        取消: Arc::new(AtomicBool::new(false)),
                        受信: None,
                        生受信: Vec::new(),
                        結果: None,
                        実行期限: None,
                        単調開始: None,
                        単調応答Millis: None,
                    },
                );
                Ok(body)
            }
            "対話承認待ち" => {
                空入力(値)?;
                Ok(
                    json!({"要求": self.作業.values().filter(|v| v.状態 == "承認待ち").map(|v| {
                        if v.評価隔離 {
                            // C5のprivate Case inputは、owner承認待ち一覧にも再投影しない。
                            // 承認に必要なrequest ID/hash、Runtime、期限だけを返す。
                            json!({
                                "要求": {
                                    "要求ID": v.要求.要求ID,
                                    "実行系ID": v.要求.実行系ID,
                                },
                                "要求hash": v.要求hash,
                                "期限": v.作成時刻 + 300,
                                "評価隔離": true,
                            })
                        } else {
                            json!({
                                "要求": v.要求,
                                "要求hash": v.要求hash,
                                "期限": v.作成時刻 + 300,
                                "接続先": self.実行系[&v.要求.実行系ID].接続対象(),
                            })
                        }
                    }).collect::<Vec<_>>()}),
                )
            }
            "対話承認" => {
                let 指定: 承認指定 = 読取(値)?;
                if !["none", "hash_only", "summary", "redacted", "full"]
                    .contains(&指定.表示範囲.as_str())
                {
                    return Err(対話失敗::要求不正);
                }
                if self.作業.values().filter(|v| v.受信.is_some()).count() >= 8 {
                    return Err(対話失敗::要求不正);
                }
                let work = self.作業.get_mut(&指定.要求ID).ok_or(対話失敗::要求不正)?;
                if work.状態 != "承認待ち"
                    || work.要求hash != 指定.要求hash
                    || 現在 >= work.作成時刻 + 300
                {
                    return Err(対話失敗::権限拒否);
                }
                let adapter = Arc::clone(
                    self.実行系
                        .get(&work.要求.実行系ID)
                        .ok_or(対話失敗::実行系不在)?,
                );
                let 理由 = if work.評価隔離 {
                    format!(
                        "対話送信承認 Capability=対話送信 Permission={} Approval={} 表示範囲={} 評価隔離=true RecoveryAction=接続再確認",
                        work.要求.実行系ID, work.要求hash, 指定.表示範囲,
                    )
                } else {
                    format!(
                        "対話送信承認 Capability=対話送信 Permission={}:{} Approval={} 表示範囲={} RecoveryAction=接続再確認",
                        work.要求.実行系ID,
                        adapter.接続対象(),
                        work.要求hash,
                        指定.表示範囲,
                    )
                };
                work.開始監査ID = Some(監査(&理由, &work.要求.要求ID, &work.要求hash)?);
                work.開始時刻 = Some(現在);
                work.表示範囲 = 指定.表示範囲;
                let 要求 = work.要求.clone();
                let 取消 = Arc::clone(&work.取消);
                let 単調開始 = Instant::now();
                let 期限 =
                    単調開始 + Duration::from_secs(10.min((work.作成時刻 + 300 - 現在) as u64));
                work.単調開始 = Some(単調開始);
                work.単調応答Millis = None;
                work.実行期限 = Some(期限);
                let (送信, 受信) = mpsc::sync_channel(1);
                work.状態 = "実行中";
                if std::thread::Builder::new()
                    .name("実行系対話".into())
                    .spawn(move || {
                        let mut 生受信 = Vec::new();
                        let 結果 = if 取消.load(Ordering::SeqCst) {
                            Err(対話失敗::取消)
                        } else {
                            adapter.応答(&要求, &取消, 期限, &mut 生受信)
                        };
                        let _ = 送信.send(受信結果 {
                            結果,
                            生受信,
                            応答完了: Some(Instant::now()),
                        });
                    })
                    .is_err()
                {
                    work.状態 = "完了";
                    work.結果 = Some(Err(対話失敗::通信失敗));
                    if let Some(s) = self.セッション.get_mut(&work.要求.対話セッションID)
                    {
                        s.状態 = "中止後隔離".into();
                    }
                    work.終了監査ID = Some(
                        監査("対話worker起動失敗", &work.要求.要求ID, &work.要求hash).map_err(
                            |e| {
                                work.状態 = "監査失敗";
                                e
                            },
                        )?,
                    );
                    work.終了時刻 = Some(現在);
                    return Err(対話失敗::通信失敗);
                }
                work.受信 = Some(受信);
                work.状態 = "実行中";
                Ok(json!({"要求ID": 指定.要求ID, "状態": "実行中"}))
            }
            "対話取得" => {
                let 指定: 要求指定 = 読取(値)?;
                let work = self.作業.get(&指定.要求ID).ok_or(対話失敗::要求不正)?;
                if work.状態 == "監査失敗" {
                    return Err(対話失敗::監査失敗);
                }
                if work.評価隔離 {
                    return Err(対話失敗::権限拒否);
                }
                Ok(
                    json!({"要求ID": 指定.要求ID, "状態": work.状態, "結果": work.結果.as_ref().map(|r| 表示射影(work, r)), "実行記録": 実行記録(work)}),
                )
            }
            "対話中止" => {
                let 指定: 要求指定 = 読取(値)?;
                let work = self.作業.get_mut(&指定.要求ID).ok_or(対話失敗::要求不正)?;
                if !matches!(work.状態, "承認待ち" | "実行中") {
                    return Err(対話失敗::要求不正);
                }
                let 終了監査ID = 監査(
                    "対話中止 実行系の停止は保証しない",
                    &指定.要求ID,
                    &work.要求hash,
                )?;
                work.取消.store(true, Ordering::SeqCst);
                work.状態 = "中止";
                work.終了時刻 = Some(現在);
                work.終了監査ID = Some(終了監査ID);
                work.結果 = Some(Err(対話失敗::取消));
                work.単調応答Millis = None;
                self.セッション
                    .get_mut(&work.要求.対話セッションID)
                    .ok_or(対話失敗::セッション不一致)?
                    .状態 = "中止後隔離".into();
                Ok(json!({"要求ID": 指定.要求ID, "状態": "中止"}))
            }
            "対話終了" => {
                let 指定: 終了指定 = 読取(値)?;
                if !self.セッション.contains_key(&指定.対話セッションID) {
                    return Err(対話失敗::セッション不一致);
                }
                if self
                    .作業
                    .values()
                    .any(|v| v.要求.対話セッションID == 指定.対話セッションID && v.受信.is_some())
                {
                    return Err(対話失敗::セッション不一致);
                }
                監査(
                    "対話終了",
                    &指定.対話セッションID,
                    &sha256_tagged(指定.対話セッションID.as_bytes()),
                )?;
                self.作業
                    .retain(|_, v| v.要求.対話セッションID != 指定.対話セッションID);
                self.セッション.remove(&指定.対話セッションID);
                Ok(json!({"対話セッションID": 指定.対話セッションID, "状態": "終了"}))
            }
            _ => Err(対話失敗::要求不正),
        };
        self.記録保存(監査)?;
        result
    }

    fn 記録保存(&mut self, 監査: &mut 監査器<'_>) -> Result<(), 対話失敗> {
        for work in self.作業.values_mut() {
            if work.状態 == "監査失敗" {
                continue;
            }
            if !work.保存済み結果証跡 {
                if let Some(proof) = 結果証跡(work) {
                    let encoded = proof.to_string();
                    if 監査(
                        &format!("対話結果証跡:{encoded}"),
                        &work.要求.要求ID,
                        &sha256_tagged(encoded.as_bytes()),
                    )
                    .is_err()
                    {
                        work.取消.store(true, Ordering::SeqCst);
                        work.状態 = "監査失敗";
                        work.結果 = Some(Err(対話失敗::監査失敗));
                        if let Some(session) = self.セッション.get_mut(&work.要求.対話セッションID)
                        {
                            session.状態 = "中止後隔離".into();
                        }
                        return Err(対話失敗::監査失敗);
                    }
                    work.保存済み結果証跡 = true;
                }
            }
            let (状態, 失敗分類) = match &work.結果 {
                Some(Ok(v)) => (if v.保留 { "保留" } else { "成功" }, None),
                Some(Err(e)) => (
                    if *e == 対話失敗::取消 {
                        "中止"
                    } else {
                        "失敗"
                    },
                    Some(e.分類()),
                ),
                None => (work.状態, None),
            };
            let body = json!({"版":2, "状態":状態, "失敗分類":失敗分類,
                "実行記録":実行記録(work), "入力概要":入力概要(work)})
            .to_string();
            let hash = sha256_tagged(body.as_bytes());
            if work.保存済み記録hash.as_ref() == Some(&hash) {
                continue;
            }
            if 監査(&format!("対話実行記録:{body}"), &work.要求.要求ID, &hash).is_err()
            {
                work.取消.store(true, Ordering::SeqCst);
                work.状態 = "監査失敗";
                work.結果 = Some(Err(対話失敗::監査失敗));
                if let Some(session) = self.セッション.get_mut(&work.要求.対話セッションID)
                {
                    session.状態 = "中止後隔離".into();
                }
                return Err(対話失敗::監査失敗);
            }
            work.保存済み記録hash = Some(hash);
        }
        Ok(())
    }

    fn 進捗反映(
        &mut self, 現在: i64, 監査: &mut 監査器<'_>
    ) -> Result<(), 対話失敗> {
        for work in self.作業.values_mut() {
            let 完了 = if let Some(受信) = &work.受信 {
                match 受信.try_recv() {
                    Ok(r) => Some(r),
                    Err(mpsc::TryRecvError::Disconnected) => Some(受信結果 {
                        結果: Err(対話失敗::通信失敗),
                        生受信: Vec::new(),
                        応答完了: None,
                    }),
                    Err(mpsc::TryRecvError::Empty) => None,
                }
            } else if work.状態 == "承認待ち" && 現在 >= work.作成時刻 + 300 {
                Some(受信結果 {
                    結果: Err(対話失敗::期限超過),
                    生受信: Vec::new(),
                    応答完了: None,
                })
            } else {
                None
            };
            let 期限内応答 = 完了
                .as_ref()
                .and_then(|結果| 結果.応答完了)
                .is_some_and(|応答完了| work.実行期限.map_or(true, |期限| 応答完了 <= 期限));
            let 期限超過 = work.状態 == "実行中"
                && work.実行期限.is_some_and(|期限| {
                    match 完了.as_ref().and_then(|結果| 結果.応答完了) {
                        Some(応答完了) => 応答完了 > 期限,
                        None => Instant::now() >= 期限,
                    }
                });
            if 期限超過 && !期限内応答 {
                work.取消.store(true, Ordering::SeqCst);
                work.結果 = Some(Err(対話失敗::期限超過));
                work.状態 = "完了";
                work.単調応答Millis = None;
                if let Some(s) = self.セッション.get_mut(&work.要求.対話セッションID)
                {
                    s.状態 = "中止後隔離".into();
                }
                let 終了監査 = 監査("対話期限超過", &work.要求.要求ID, &work.要求hash);
                if 終了監査.is_err() {
                    work.状態 = "監査失敗";
                    work.結果 = Some(Err(対話失敗::監査失敗));
                    return Err(対話失敗::監査失敗);
                }
                work.終了時刻 = Some(現在);
                work.終了監査ID = 終了監査.ok();
            }
            if let Some(mut 受信結果) = 完了 {
                work.受信 = None;
                work.単調応答Millis = if work.結果.is_none() && 期限内応答 {
                    match (work.単調開始, 受信結果.応答完了) {
                        (Some(開始), Some(応答完了)) => Some(
                            応答完了
                                .saturating_duration_since(開始)
                                .as_millis()
                                .min(u128::from(u64::MAX)) as u64,
                        ),
                        _ => None,
                    }
                } else {
                    None
                };
                if 受信結果.生受信.len() > 4
                    || 受信結果.生受信.iter().any(|v| v.len() > 1024 * 1024)
                {
                    受信結果.結果 = Err(対話失敗::応答不正);
                    受信結果.生受信.truncate(4);
                    for v in &mut 受信結果.生受信 {
                        v.truncate(1024 * 1024);
                    }
                }
                work.生受信 = 受信結果.生受信;
                let 結果 = 受信結果.結果.and_then(|v| 結果検査(&work.要求, v));
                let hashes: Vec<_> = work.生受信.iter().map(|v| sha256_tagged(v)).collect();
                let hash = sha256_tagged(
                    json!({"要求hash": work.要求hash, "受信hash": hashes})
                        .to_string()
                        .as_bytes(),
                );
                let 種別 = if work.結果.is_some() {
                    "採用終了後応答破棄"
                } else {
                    "対話完了"
                };
                let 終了監査 = 監査(種別, &work.要求.要求ID, &hash);
                if 終了監査.is_err() {
                    work.取消.store(true, Ordering::SeqCst);
                    work.状態 = "監査失敗";
                    work.結果 = Some(Err(対話失敗::監査失敗));
                    if let Some(s) = self.セッション.get_mut(&work.要求.対話セッションID)
                    {
                        s.状態 = "中止後隔離".into();
                    }
                    return Err(対話失敗::監査失敗);
                }
                if work.結果.is_none() {
                    work.終了時刻 = Some(現在);
                    work.終了監査ID = 終了監査.ok();
                    if 結果.is_err() {
                        if let Some(s) = self.セッション.get_mut(&work.要求.対話セッションID)
                        {
                            s.状態 = "中止後隔離".into();
                        }
                    }
                    work.状態 = "完了";
                    work.結果 = Some(結果);
                }
            }
        }
        self.記録保存(監査)?;
        self.失効資源解放();
        Ok(())
    }
}

fn 実行記録(work: &作業) -> Value {
    json!({"要求ID": work.要求.要求ID, "実行系ID": work.要求.実行系ID,
        "対話セッションID": work.要求.対話セッションID, "作成時刻": work.作成時刻,
        "開始時刻": work.開始時刻, "終了時刻": work.終了時刻,
        "作成監査ID": work.作成監査ID, "開始監査ID": work.開始監査ID,
        "終了監査ID": work.終了監査ID})
}

/// 永続履歴のmetadata用。入力本文を復元・公開する権限や正しさの証拠にはしない。
fn 入力概要(work: &作業) -> Value {
    json!({"表示範囲":"hash_only", "入力hash":sha256_tagged(work.要求.入力.as_bytes())})
}

fn 結果検査(要求: &対話要求, v: 実行結果) -> Result<実行結果, 対話失敗> {
    if v.対話セッションID != 要求.対話セッションID {
        return Err(対話失敗::セッション不一致);
    }
    let hex = |s: &str, n: usize| {
        s.len() == n
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    if v.本文.chars().count() > 65536
        || v.参照.len() > 64
        || v.参照.iter().any(|s| s.chars().count() > 2048)
        || v.能力.len() > 64
        || v.能力.iter().any(|s| s.chars().count() > 256)
        || v.経路.chars().count() > 256
        || !hex(&v.追跡ID, 32)
        || !v
            .追跡hash
            .strip_prefix("sha256:")
            .is_some_and(|s| hex(s, 64))
        || v.生応答.is_empty()
        || v.生応答.len() > 1024 * 1024
    {
        return Err(対話失敗::応答不正);
    }
    Ok(v)
}

fn 結果証跡(work: &作業) -> Option<Value> {
    let result = work.結果.as_ref()?.as_ref().ok()?;
    let ended = work.終了監査ID.as_ref()?;
    if work.表示範囲 == "none" {
        return None;
    }
    let full = work.表示範囲 == "full";
    let hash = |v: Value| sha256_tagged(v.to_string().as_bytes());
    Some(
        json!({"版":1,"要求ID":work.要求.要求ID,"対話セッションID":work.要求.対話セッションID,
        "実行系ID":work.要求.実行系ID,"要求hash":work.要求hash,"終了監査ID":ended,
        "表示範囲":work.表示範囲,"応答hash":sha256_tagged(&result.生応答),
        "能力申告hash":full.then(|| hash(json!(result.能力))),
        "経路申告hash":full.then(|| hash(json!(result.経路))),
        "追跡参照hash":full.then(|| hash(json!({"追跡ID":result.追跡ID,"追跡hash":result.追跡hash}))),
        "証拠種別":"INTERNAL_STATE"}),
    )
}

fn 表示射影(work: &作業, 結果: &Result<実行結果, 対話失敗>) -> Value {
    let mut body = json!({"要求ID": work.要求.要求ID, "実行系ID": work.要求.実行系ID, "対話セッションID": work.要求.対話セッションID,
        "状態": "成功", "表示範囲": work.表示範囲, "本文": "", "参照": [], "能力": [], "経路": "", "追跡ID": "", "追跡hash": "", "応答hash": "", "失敗分類": "", "復旧": ""});
    match 結果 {
        Ok(v) => {
            if v.保留 {
                body["状態"] = json!("保留");
            }
            if work.表示範囲 != "none" {
                body["応答hash"] = json!(sha256_tagged(&v.生応答));
            }
            if work.表示範囲 == "full" {
                body["本文"] = json!(v.本文);
                body["参照"] = json!(v.参照);
                body["能力"] = json!(v.能力);
                body["経路"] = json!(v.経路);
                body["追跡ID"] = json!(v.追跡ID);
                body["追跡hash"] = json!(v.追跡hash);
            }
        }
        Err(e) => {
            body["状態"] = json!(if *e == 対話失敗::取消 {
                "中止"
            } else {
                "失敗"
            });
            body["失敗分類"] = json!(e.分類());
            body["復旧"] = json!(e.復旧());
        }
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn 対話開始監査hashは既存session射影と一致する() {
        let session = セッション {
            対話セッションID: "session-fixture".into(),
            実行系ID: "runtime-fixture".into(),
            状態: "利用中".into(),
        };
        let legacy_body = serde_json::to_value(&session).expect("Session射影");
        assert_eq!(
            対話開始監査hash(&session.対話セッションID, &session.実行系ID),
            sha256_tagged(legacy_body.to_string().as_bytes())
        );
    }

    struct 試験Adapter {
        回数: Arc<AtomicUsize>,
        失敗: bool,
        別session: bool,
        遅延: bool,
    }
    impl 実行系Adapter for 試験Adapter {
        fn 接続対象(&self) -> String {
            "試験専用".into()
        }
        fn 応答(
            &self,
            r: &対話要求,
            _: &AtomicBool,
            _: Instant,
            raw: &mut Vec<Vec<u8>>,
        ) -> Result<実行結果, 対話失敗> {
            self.回数.fetch_add(1, Ordering::SeqCst);
            if self.遅延 {
                std::thread::sleep(Duration::from_millis(100));
            }
            raw.push(b"raw-private".to_vec());
            if self.失敗 {
                return Err(対話失敗::通信失敗);
            }
            Ok(実行結果 {
                対話セッションID: if self.別session {
                    "別session".into()
                } else {
                    r.対話セッションID.clone()
                },
                本文: r.入力.clone(),
                参照: vec!["秘密参照".into()],
                能力: vec!["fixture".into()],
                経路: "fixture".into(),
                追跡ID: "a".repeat(32),
                追跡hash: format!("sha256:{}", "b".repeat(64)),
                保留: false,
                生応答: b"raw-private".to_vec(),
            })
        }
    }
    fn 準備(失敗: bool, 別session: bool, 遅延: bool) -> (対話制御, Arc<AtomicUsize>) {
        let mut c = 対話制御::default();
        let n = Arc::new(AtomicUsize::new(0));
        c.登録(
            "left",
            Arc::new(試験Adapter {
                回数: n.clone(),
                失敗,
                別session,
                遅延,
            }),
        )
        .unwrap();
        (c, n)
    }
    fn 操作(
        c: &mut 対話制御, op: &str, v: Value, owner: bool
    ) -> Result<Value, 対話失敗> {
        c.操作(
            op,
            &v,
            owner,
            100,
            &mut |_, _, _| Ok("fixture-audit".into()),
        )
    }
    fn 開始(c: &mut 対話制御, id: &str) -> String {
        操作(c, "対話開始", json!({"実行系ID":id}), false).unwrap()["対話セッションID"]
            .as_str()
            .unwrap()
            .into()
    }
    fn 要求(c: &mut 対話制御, s: &str) -> Value {
        操作(
            c,
            "対話送信",
            json!({"対話セッションID":s,"入力":"こんにちは"}),
            false,
        )
        .unwrap()
    }
    fn 承認(p: &Value, scope: &str) -> Value {
        json!({"要求ID":p["要求ID"],"要求hash":p["要求hash"],"表示範囲":scope})
    }
    fn 評価成功受信(対話セッションID: String, 応答完了: Instant) -> 受信結果 {
        受信結果 {
            結果: Ok(実行結果 {
                対話セッションID,
                本文: "評価結果".into(),
                参照: Vec::new(),
                能力: Vec::new(),
                経路: "fixture".into(),
                追跡ID: "a".repeat(32),
                追跡hash: format!("sha256:{}", "b".repeat(64)),
                保留: false,
                生応答: b"evaluation-raw".to_vec(),
            }),
            生受信: vec![b"evaluation-raw".to_vec()],
            応答完了: Some(応答完了),
        }
    }
    fn 完了(c: &mut 対話制御, p: &Value) -> Value {
        for _ in 0..200 {
            let v = 操作(c, "対話取得", json!({"要求ID":p["要求ID"]}), false).unwrap();
            if v["状態"] == "完了" {
                assert!(v["実行記録"]["終了時刻"].is_i64());
                assert!(v["実行記録"]["終了監査ID"].is_string());
                assert_eq!(v["実行記録"]["要求ID"], p["要求ID"]);
                return v["結果"].clone();
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("対話完了の待機期限");
    }

    #[test]
    fn 対話終了はsession上限を回復する() {
        let (mut c, _) = 準備(false, false, false);
        let sessions = (0..64)
            .map(|_| 開始(&mut c, "left"))
            .collect::<Vec<_>>();

        assert_eq!(c.評価要求可能数(), 0);
        assert_eq!(
            操作(&mut c, "対話開始", json!({"実行系ID":"left"}), false),
            Err(対話失敗::要求不正)
        );

        操作(
            &mut c,
            "対話終了",
            json!({"対話セッションID": sessions[0]}),
            false,
        )
        .unwrap();
        assert_eq!(c.評価要求可能数(), 1);
        assert!(!開始(&mut c, "left").is_empty());
    }

    #[test]
    fn 実行系IDはIPC契約と同じ境界を使う() {
        for accepted in ["local", "local-1", "A.b_c-9"] {
            assert!(実行系ID妥当(accepted), "許可するID: {accepted}");
        }
        for rejected in [
            "",
            ".local",
            "_local",
            "local runtime",
            "local\nnext",
            "実行系",
            "pid:1234",
            "local/..",
        ] {
            assert!(!実行系ID妥当(rejected), "拒否するID: {rejected:?}");
        }
        assert!(!実行系ID妥当(&"a".repeat(129)));
    }

    #[test]
    fn 資源統計は高精度時計を持たず完了数と失敗数だけを返す() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        操作(&mut c, "対話承認", 承認(&pending, "full"), true).unwrap();
        完了(&mut c, &pending);

        let stats = c.資源統計("left");
        assert_eq!(stats.完了要求数, 1);
        assert_eq!(stats.失敗要求数, 0);
    }

    #[test]
    fn 評価隔離済み対話は通常取得を拒否し内部進捗だけを返す() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        let request_id = pending["要求ID"].as_str().unwrap();

        assert!(matches!(
            c.評価進捗(request_id, 100, &mut |_, _, _| Ok("fixture-audit".into())),
            Err(対話失敗::要求不正)
        ));
        assert_eq!(c.評価隔離(request_id), Ok(()));
        assert_eq!(c.評価隔離(request_id), Err(対話失敗::要求不正));
        for owner in [false, true] {
            assert_eq!(
                操作(&mut c, "対話取得", json!({"要求ID": request_id}), owner),
                Err(対話失敗::権限拒否)
            );
        }
        let pending_progress = c
            .評価進捗(request_id, 100, &mut |_, _, _| Ok("fixture-audit".into()))
            .unwrap();
        assert_eq!(pending_progress.状態, "承認待ち");
        assert!(pending_progress.結果.is_none());

        let mut reasons = Vec::new();
        c.操作(
            "対話承認",
            &承認(&pending, "full"),
            true,
            101,
            &mut |reason, _, _| {
                reasons.push(reason.to_owned());
                Ok("fixture-audit".into())
            },
        )
        .unwrap();
        let evaluation_reason = reasons
            .iter()
            .find(|reason| reason.starts_with("対話送信承認"))
            .unwrap();
        assert!(evaluation_reason.contains("評価隔離=true"));
        assert!(!evaluation_reason.contains("試験専用"));

        let limit = Instant::now() + Duration::from_secs(2);
        let completed = loop {
            let progress = c
                .評価進捗(request_id, 102, &mut |_, _, _| Ok("fixture-audit".into()))
                .unwrap();
            if progress.状態 == "完了" {
                break progress;
            }
            assert!(Instant::now() < limit, "評価対話完了の待機期限");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(completed.結果.unwrap()["本文"], "こんにちは");
        assert_eq!(
            c.保存対象(&json!({"要求ID": request_id, "要求hash": pending["要求hash"]})),
            Err(対話失敗::権限拒否)
        );
        for owner in [false, true] {
            assert_eq!(
                操作(&mut c, "対話取得", json!({"要求ID": request_id}), owner),
                Err(対話失敗::権限拒否)
            );
        }

        let normal_session = 開始(&mut c, "left");
        let normal_pending = 要求(&mut c, &normal_session);
        let mut normal_reasons = Vec::new();
        c.操作(
            "対話承認",
            &承認(&normal_pending, "full"),
            true,
            103,
            &mut |reason, _, _| {
                normal_reasons.push(reason.to_owned());
                Ok("fixture-audit".into())
            },
        )
        .unwrap();
        assert!(normal_reasons
            .iter()
            .any(|reason| reason.contains("Permission=left:試験専用")
                && !reason.contains("評価隔離=true")));
    }

    #[test]
    fn 評価遅延はworker応答完了時刻から求めpoll待機を含めない() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        let request_id = pending["要求ID"].as_str().unwrap();
        c.評価隔離(request_id).unwrap();

        let (sender, receiver) = mpsc::sync_channel(1);
        let started = Instant::now() - Duration::from_millis(600);
        let completed = Instant::now() - Duration::from_millis(400);
        {
            let work = c.作業.get_mut(request_id).unwrap();
            work.状態 = "実行中";
            work.単調開始 = Some(started);
            work.実行期限 = Some(Instant::now() + Duration::from_secs(1));
            work.受信 = Some(receiver);
        }
        sender.send(評価成功受信(session, completed)).unwrap();
        std::thread::sleep(Duration::from_millis(250));

        let progress = c
            .評価進捗(request_id, 101, &mut |_, _, _| Ok("fixture-audit".into()))
            .unwrap();
        let latency = progress.単調応答Millis.unwrap();
        assert!(
            latency >= 100 && latency < 350,
            "poll時刻を含む遅延: {latency}"
        );
    }

    #[test]
    fn 期限前に完了した評価対話は期限後pollでも採用する() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        let request_id = pending["要求ID"].as_str().unwrap();
        c.評価隔離(request_id).unwrap();

        let (sender, receiver) = mpsc::sync_channel(1);
        let now = Instant::now();
        let deadline = now - Duration::from_millis(100);
        let response_completed = now - Duration::from_millis(200);
        {
            let work = c.作業.get_mut(request_id).unwrap();
            work.状態 = "実行中";
            work.単調開始 = Some(now - Duration::from_millis(300));
            work.実行期限 = Some(deadline);
            work.受信 = Some(receiver);
        }
        sender
            .send(評価成功受信(session, response_completed))
            .unwrap();

        let mut reasons = Vec::new();
        let progress = c
            .評価進捗(request_id, 101, &mut |reason, _, _| {
                reasons.push(reason.to_owned());
                Ok("fixture-audit".into())
            })
            .unwrap();
        assert_eq!(progress.状態, "完了");
        assert_eq!(progress.結果.unwrap()["状態"], "成功");
        assert!(progress.単調応答Millis.is_some());
        assert!(!reasons.iter().any(|reason| reason == "対話期限超過"));
    }

    #[test]
    fn 中止後にworker応答を回収しても評価遅延を残さない() {
        let (mut c, calls) = 準備(false, false, true);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        let request_id = pending["要求ID"].as_str().unwrap();
        c.評価隔離(request_id).unwrap();
        操作(&mut c, "対話承認", 承認(&pending, "full"), true).unwrap();
        let limit = Instant::now() + Duration::from_secs(2);
        while calls.load(Ordering::SeqCst) == 0 {
            assert!(Instant::now() < limit, "worker開始待機期限");
            std::thread::sleep(Duration::from_millis(1));
        }
        操作(&mut c, "対話中止", json!({"要求ID": request_id}), false).unwrap();
        std::thread::sleep(Duration::from_millis(150));

        let progress = c
            .評価進捗(request_id, 101, &mut |_, _, _| Ok("fixture-audit".into()))
            .unwrap();
        assert_eq!(progress.状態, "中止");
        assert_eq!(progress.単調応答Millis, None);
    }

    #[test]
    fn 保存対象は全文の完了と確定記録を要求する() {
        for (scope, fail) in [
            ("none", false),
            ("hash_only", false),
            ("summary", false),
            ("redacted", false),
            ("full", true),
            ("full", false),
        ] {
            let (mut c, _) = 準備(fail, false, false);
            let session = 開始(&mut c, "left");
            let p = 要求(&mut c, &session);
            let select = json!({"要求ID":p["要求ID"],"要求hash":p["要求hash"]});
            assert!(c.保存対象(&select).is_err());
            操作(&mut c, "対話承認", 承認(&p, scope), true).unwrap();
            完了(&mut c, &p);
            if scope != "full" || fail {
                assert!(c.保存対象(&select).is_err());
                continue;
            }
            let content = c.保存対象(&select).unwrap();
            assert_eq!(content["要求"]["入力"], "こんにちは");
            assert_eq!(content["結果"]["本文"], "こんにちは");
            assert!(!content.to_string().contains("raw-private"));
            assert!(c
                .保存対象(&json!({"要求ID":p["要求ID"],"要求hash":"wrong"}))
                .is_err());
            let mut bad = select.clone();
            bad["本文"] = json!("注入");
            assert!(c.保存対象(&bad).is_err());
            c.作業
                .get_mut(p["要求ID"].as_str().unwrap())
                .unwrap()
                .保存済み結果証跡 = false;
            assert!(c.保存対象(&select).is_err());
            c.資格隔離(&[session]);
            assert!(c.保存対象(&select).is_err());
        }
    }
    #[test]
    fn 結果証跡は表示範囲を保持し一度だけ保存する() {
        for (scope, fail) in [
            ("none", false),
            ("hash_only", false),
            ("summary", false),
            ("redacted", false),
            ("full", false),
            ("full", true),
        ] {
            let (mut c, _) = 準備(fail, false, false);
            let session = 開始(&mut c, "left");
            let p = 要求(&mut c, &session);
            操作(&mut c, "対話承認", 承認(&p, scope), true).unwrap();
            let mut proofs = Vec::new();
            let mut finished = false;
            for _ in 0..200 {
                let response = c
                    .操作(
                        "対話取得",
                        &json!({"要求ID":p["要求ID"]}),
                        false,
                        101,
                        &mut |reason, _, hash| {
                            if let Some(body) = reason.strip_prefix("対話結果証跡:") {
                                assert_eq!(sha256_tagged(body.as_bytes()), hash);
                                assert!(!body.contains("fixture") && !body.contains("raw-private"));
                                proofs.push(serde_json::from_str::<Value>(body).unwrap());
                            }
                            Ok("proof-audit".into())
                        },
                    )
                    .unwrap();
                if response["状態"] == "完了" {
                    finished = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(finished);
            c.操作(
                "対話取得",
                &json!({"要求ID":p["要求ID"]}),
                false,
                102,
                &mut |reason, _, _| {
                    assert!(!reason.starts_with("対話結果証跡:"));
                    Ok("next-audit".into())
                },
            )
            .unwrap();
            if scope == "none" || fail {
                assert!(proofs.is_empty());
            } else {
                assert_eq!(proofs.len(), 1);
                let proof = &proofs[0];
                assert_eq!(proof["要求hash"], p["要求hash"]);
                assert_eq!(proof["応答hash"], sha256_tagged(b"raw-private"));
                assert_eq!(proof["能力申告hash"].is_string(), scope == "full");
                assert_eq!(proof["経路申告hash"].is_string(), scope == "full");
                assert_eq!(proof["追跡参照hash"].is_string(), scope == "full");
            }
        }
    }
    #[test]
    fn 結果証跡の書込失敗で本文返却と再送を止める() {
        let (mut c, _) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let p = 要求(&mut c, &session);
        操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
        let mut rejected = false;
        for _ in 0..200 {
            let response = c.操作(
                "対話取得",
                &json!({"要求ID":p["要求ID"]}),
                false,
                101,
                &mut |reason, _, _| {
                    if reason.starts_with("対話結果証跡:") {
                        Err(対話失敗::監査失敗)
                    } else {
                        Ok("proof-audit".into())
                    }
                },
            );
            if response == Err(対話失敗::監査失敗) {
                rejected = true;
                break;
            }
            assert_ne!(response.unwrap()["状態"], "完了");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(rejected);
        assert_eq!(
            操作(&mut c, "対話取得", json!({"要求ID":p["要求ID"]}), false),
            Err(対話失敗::監査失敗)
        );
        assert!(操作(
            &mut c,
            "対話送信",
            json!({"対話セッションID":session,"入力":"再送しない"}),
            false
        )
        .is_err());
    }

    #[test]
    fn 履歴保存は変更時だけ行い書込失敗で採用を止める() {
        let (mut c, _) = 準備(false, false, true);
        let session = 開始(&mut c, "left");
        let mut records = Vec::new();
        let mut audit = |reason: &str, _: &str, hash: &str| {
            if let Some(body) = reason.strip_prefix("対話実行記録:") {
                assert_eq!(sha256_tagged(body.as_bytes()), hash);
                records.push(serde_json::from_str::<Value>(body).unwrap());
            }
            Ok("history-test-event".into())
        };
        let p = c
            .操作(
                "対話送信",
                &json!({"対話セッションID":session,"入力":"保存しない本文"}),
                false,
                100,
                &mut audit,
            )
            .unwrap();
        for _ in 0..3 {
            c.操作(
                "対話取得",
                &json!({"要求ID":p["要求ID"]}),
                false,
                101,
                &mut audit,
            )
            .unwrap();
        }
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["版"], 2);
        assert_eq!(records[0]["状態"], "承認待ち");
        assert_eq!(
            records[0]["入力概要"],
            json!({"表示範囲":"hash_only",
            "入力hash":sha256_tagged("保存しない本文".as_bytes())})
        );
        assert!(!records[0].to_string().contains("保存しない本文"));
        assert_eq!(
            c.操作(
                "対話承認",
                &承認(&p, "full"),
                true,
                110,
                &mut |reason, _, _| if reason.starts_with("対話実行記録:") {
                    Err(対話失敗::監査失敗)
                } else {
                    Ok("approved-event".into())
                }
            ),
            Err(対話失敗::監査失敗)
        );
        assert_eq!(
            操作(&mut c, "対話取得", json!({"要求ID":p["要求ID"]}), false),
            Err(対話失敗::監査失敗)
        );
        assert!(操作(
            &mut c,
            "対話送信",
            json!({"対話セッションID":session,"入力":"再送しない"}),
            false
        )
        .is_err());
    }

    #[test]
    fn 実行記録は未実行と終了を区別し遅延応答で上書きしない() {
        for approve in [false, true] {
            let (mut c, _) = 準備(false, false, true);
            let session = 開始(&mut c, "left");
            let p = 要求(&mut c, &session);
            let id = p["要求ID"].as_str().unwrap();
            let before = 実行記録(&c.作業[id]);
            assert!(before["開始時刻"].is_null());
            assert!(before["終了監査ID"].is_null());
            if approve {
                c.操作(
                    "対話承認",
                    &承認(&p, "full"),
                    true,
                    110,
                    &mut |_, _, _| Ok("approved-event".into()),
                )
                .unwrap();
            }
            c.操作(
                "対話中止",
                &json!({"要求ID":id}),
                false,
                120,
                &mut |_, _, _| Ok("cancel-event".into()),
            )
            .unwrap();
            let record = 実行記録(&c.作業[id]);
            assert_eq!(record["作成時刻"], 100);
            assert_eq!(
                record["開始時刻"],
                if approve { json!(110) } else { Value::Null }
            );
            assert_eq!(
                record["開始監査ID"],
                if approve {
                    json!("approved-event")
                } else {
                    Value::Null
                }
            );
            assert_eq!(record["終了時刻"], 120);
            assert_eq!(record["終了監査ID"], "cancel-event");
            std::thread::sleep(Duration::from_millis(150));
            c.進捗反映(130, &mut |_, _, _| Ok("late-event".into()))
                .unwrap();
            assert_eq!(実行記録(&c.作業[id]), record);
            assert!(!record.to_string().contains("こんにちは"));
        }
        let (mut c, count) = 準備(false, false, false);
        let session = 開始(&mut c, "left");
        let p = 要求(&mut c, &session);
        let v = c
            .操作(
                "対話取得",
                &json!({"要求ID":p["要求ID"]}),
                false,
                400,
                &mut |_, _, _| Ok("expired-event".into()),
            )
            .unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(v["実行記録"]["開始時刻"].is_null());
        assert_eq!(v["実行記録"]["終了時刻"], 400);
        assert_eq!(v["実行記録"]["終了監査ID"], "expired-event");
        assert_eq!(v["結果"]["失敗分類"], "期限超過");
    }

    #[test]
    fn 失効後の遅延応答は監査してから資源解放する() {
        let (mut c, count) = 準備(false, false, true);
        let session = 開始(&mut c, "left");
        let pending = 要求(&mut c, &session);
        操作(&mut c, "対話承認", 承認(&pending, "full"), true).unwrap();
        let limit = Instant::now() + Duration::from_secs(2);
        while count.load(Ordering::SeqCst) == 0 {
            assert!(Instant::now() < limit, "worker開始待機期限");
            std::thread::sleep(Duration::from_millis(1));
        }
        c.資格隔離(&[session.clone()]);
        assert!(c.作業.values().any(|v| v.受信.is_some()));
        let mut discarded = false;
        for _ in 0..200 {
            c.進捗反映(100, &mut |reason, _, _| {
                if reason == "採用終了後応答破棄" {
                    discarded = true;
                }
                Ok("fixture-audit".into())
            })
            .unwrap();
            if c.作業.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(discarded);
        assert!(c.作業.is_empty());
        assert!(c.セッション.is_empty());
    }

    #[test]
    fn 端末失効の反復で対話枠を占有し続けない() {
        let (mut c, _) = 準備(false, false, false);
        for _ in 0..70 {
            let session = 開始(&mut c, "left");
            let pending = 要求(&mut c, &session);
            c.資格隔離(&[session.clone()]);
            assert!(操作(&mut c, "対話承認", 承認(&pending, "full"), true).is_err());
            assert!(c.セッション.is_empty());
            assert!(c.作業.is_empty());
        }
    }

    #[test]
    fn Adapterが期限を無視してもCoreは遅延応答を採用しない() {
        let (mut c, _) = 準備(false, false, true);
        let s = 開始(&mut c, "left");
        let p = 要求(&mut c, &s);
        操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
        c.作業
            .get_mut(p["要求ID"].as_str().unwrap())
            .unwrap()
            .実行期限 = Some(Instant::now());
        assert_eq!(完了(&mut c, &p)["失敗分類"], "期限超過");
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(完了(&mut c, &p)["失敗分類"], "期限超過");
    }
    #[test]
    fn 承認資格とhashと一回性を強制する() {
        let (mut c, n) = 準備(false, false, false);
        let s = 開始(&mut c, "left");
        let p = 要求(&mut c, &s);
        assert_eq!(n.load(Ordering::SeqCst), 0);
        assert_eq!(
            操作(&mut c, "対話承認待ち", json!({}), false),
            Err(対話失敗::権限拒否)
        );
        assert_eq!(
            操作(&mut c, "対話承認", 承認(&p, "full"), false),
            Err(対話失敗::権限拒否)
        );
        let mut bad = 承認(&p, "full");
        bad["要求hash"] = json!("sha256:wrong");
        assert_eq!(操作(&mut c, "対話承認", bad, true), Err(対話失敗::権限拒否));
        操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
        assert_eq!(完了(&mut c, &p)["本文"], "こんにちは");
        assert_eq!(
            操作(&mut c, "対話承認", 承認(&p, "full"), true),
            Err(対話失敗::権限拒否)
        );
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn 非全文scopeは本文も参照も追跡も公開しない() {
        for scope in ["none", "hash_only", "summary", "redacted"] {
            let (mut c, _) = 準備(false, false, false);
            let s = 開始(&mut c, "left");
            let p = 要求(&mut c, &s);
            操作(&mut c, "対話承認", 承認(&p, scope), true).unwrap();
            let v = 完了(&mut c, &p);
            for key in ["本文", "経路", "追跡ID", "追跡hash"] {
                assert_eq!(v[key], "");
            }
            assert_eq!(v["参照"], json!([]));
            assert_eq!(v["能力"], json!([]));
            assert_eq!(v["応答hash"] == "", scope == "none");
            assert!(!v.to_string().contains("raw-private"));
        }
    }
    #[test]
    fn 監査失敗なら送信せず結果確定失敗なら本文を返さない() {
        let (mut c, n) = 準備(false, false, false);
        let s = 開始(&mut c, "left");
        let p = 要求(&mut c, &s);
        assert_eq!(
            c.操作(
                "対話承認",
                &承認(&p, "full"),
                true,
                100,
                &mut |_, _, _| Err(対話失敗::監査失敗)
            ),
            Err(対話失敗::監査失敗)
        );
        assert_eq!(n.load(Ordering::SeqCst), 0);
        操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
        let limit = Instant::now() + Duration::from_secs(2);
        loop {
            let result = c.操作(
                "対話取得",
                &json!({"要求ID":p["要求ID"]}),
                false,
                100,
                &mut |_, _, _| Err(対話失敗::監査失敗),
            );
            if result == Err(対話失敗::監査失敗) {
                break;
            }
            assert!(Instant::now() < limit, "完了監査の待機期限");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            操作(&mut c, "対話取得", json!({"要求ID":p["要求ID"]}), false),
            Err(対話失敗::監査失敗)
        );
    }
    #[test]
    fn 期限切れと未知fieldと空白入力を拒否する() {
        let (mut c, n) = 準備(false, false, false);
        let s = 開始(&mut c, "left");
        for v in [
            json!({"対話セッションID":s,"入力":" "}),
            json!({"対話セッションID":s,"入力":"x","authority":"owner"}),
            json!({"対話セッションID":s,"入力":"x".repeat(4097)}),
        ] {
            assert_eq!(操作(&mut c, "対話送信", v, false), Err(対話失敗::要求不正));
        }
        let p = 要求(&mut c, &s);
        assert_eq!(
            c.操作(
                "対話承認",
                &承認(&p, "full"),
                true,
                400,
                &mut |_, _, _| Ok("fixture-audit".into())
            ),
            Err(対話失敗::権限拒否)
        );
        assert_eq!(n.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn 取消後の応答は保持しても採用せずsessionを隔離する() {
        for running in [false, true] {
            let (mut c, n) = 準備(false, false, true);
            let s = 開始(&mut c, "left");
            let p = 要求(&mut c, &s);
            if running {
                操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
                let limit = Instant::now() + Duration::from_secs(2);
                while n.load(Ordering::SeqCst) == 0 {
                    assert!(Instant::now() < limit, "worker開始の待機期限");
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            操作(&mut c, "対話中止", json!({"要求ID":p["要求ID"]}), false).unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let v = 操作(&mut c, "対話取得", json!({"要求ID":p["要求ID"]}), false).unwrap();
                assert_eq!(v["結果"]["状態"], "中止");
                assert_eq!(v["結果"]["本文"], "");
                if c.作業[p["要求ID"].as_str().unwrap()].受信.is_none() {
                    break;
                }
                assert!(Instant::now() < deadline, "取消後応答の受信待機期限");
                std::thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(
                操作(
                    &mut c,
                    "対話送信",
                    json!({"対話セッションID":s,"入力":"再送"}),
                    false
                ),
                Err(対話失敗::セッション不一致)
            );
            assert_eq!(n.load(Ordering::SeqCst), usize::from(running));
            if running {
                assert_eq!(c.作業[p["要求ID"].as_str().unwrap()].生受信.len(), 1);
            }
        }
    }
    #[test]
    fn 左右の成功と片側失敗と両側失敗を分離する() {
        for (left, right) in [(false, false), (true, false), (false, true), (true, true)] {
            let (mut c, _) = 準備(left, false, false);
            c.登録(
                "right",
                Arc::new(試験Adapter {
                    回数: Arc::new(AtomicUsize::new(0)),
                    失敗: right,
                    別session: false,
                    遅延: false,
                }),
            )
            .unwrap();
            let a = 開始(&mut c, "left");
            let b = 開始(&mut c, "right");
            assert_ne!(a, b);
            let p = 要求(&mut c, &a);
            let q = 要求(&mut c, &b);
            操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
            操作(&mut c, "対話承認", 承認(&q, "hash_only"), true).unwrap();
            let x = 完了(&mut c, &p);
            let y = 完了(&mut c, &q);
            assert_eq!(x["実行系ID"], "left");
            assert_eq!(y["実行系ID"], "right");
            assert_eq!(x["状態"], if left { "失敗" } else { "成功" });
            assert_eq!(y["状態"], if right { "失敗" } else { "成功" });
            assert_eq!(y["本文"], "");
        }
    }
    #[test]
    fn Adapterの別session応答をCoreでも拒否する() {
        let (mut c, _) = 準備(false, true, false);
        let s = 開始(&mut c, "left");
        let p = 要求(&mut c, &s);
        操作(&mut c, "対話承認", 承認(&p, "full"), true).unwrap();
        let v = 完了(&mut c, &p);
        assert_eq!(v["失敗分類"], "セッション不一致");
        assert_eq!(v["本文"], "");
    }
}
