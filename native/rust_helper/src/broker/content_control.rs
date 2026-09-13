use super::*;

impl Broker {
    pub(super) fn 内容閲覧処理(&mut self,id:&str,op:&str,payload:&Value,owner:bool,hash:&str)->BrokerResponse {
        #[cfg(not(windows))]
        { let _=(payload,owner); self.reject_with_payload_hash(id,op,"保管未対応","このOSの安全保管は未対応",true,hash) }
        #[cfg(windows)]
        {
            if op=="対話内容失効" && owner {self.内容閲覧.revoke();}
            let log=match self.state_store.persistent_store.as_ref().and_then(|s|s.verified_audit_log().ok()) {
                Some(v) if v==self.audit_log=>v,
                _=>{self.内容閲覧.revoke();return self.audit_store_failed_response(id,op,"監査失敗","保管監査再確認");}
            };
            if self.append_audit(id,op,"received","Capability=対話内容閲覧 Permission=保存参照照合 Approval=現在owner承認 RecoveryAction=保管監査再確認",EVIDENCE_SOURCE_INTERNAL_STATE,hash).is_err() {
                self.内容閲覧.revoke();return self.audit_store_failed_response(id,op,"監査失敗","保管監査再確認");
            }
            let Some(store)=self.protected_store.take() else {
                self.内容閲覧.revoke();return self.reject_with_payload_hash(id,op,"保管未登録","独立保管先の登録が必要",true,hash);
            };
            let mut access=std::mem::take(&mut self.内容閲覧);
            let now=self.current_epoch_seconds();
            let result=access.operate(op,payload,owner,now,&log,
                &mut |reason,digest|self.append_audit(id,op,"recorded",reason,EVIDENCE_SOURCE_INTERNAL_STATE,digest).map(|_|()).map_err(|_|"監査失敗"),
                &mut |target,digest|store.read(crate::protected_store::Purpose::History,target,digest).map(|s|s.as_bytes().to_vec()).map_err(|_|"保管読取/復号に失敗"));
            self.protected_store=Some(store);
            self.内容閲覧=access;
            match result {
                Err(reason)=>{self.内容閲覧.revoke();self.reject_with_payload_hash(id,op,"内容閲覧拒否",reason,true,hash)},
                Ok(body)=>self.内容閲覧確定(id,op,body,hash),
            }
        }
    }

    #[cfg(windows)]
    pub(super) fn 内容閲覧確定(&mut self,id:&str,op:&str,body:Value,hash:&str)->BrokerResponse {
        let event=match self.append_audit(id,op,"accepted","現在承認に限定した内容操作の結果を確定",EVIDENCE_SOURCE_INTERNAL_STATE,&sha256_tagged(body.to_string().as_bytes())) {
            Ok(v)=>v,
            Err(_)=>{self.内容閲覧.revoke();return self.audit_store_failed_response(id,op,"監査失敗","保管監査再確認");}
        };
        let now=self.current_epoch_seconds();
        if !body["grant"].is_null() && !self.内容閲覧.current(&body,now) {
            return self.reject_with_payload_hash(id,op,"期限超過","内容閲覧を再承認してください",true,hash);
        }
        BrokerResponse {request_id:id.into(),operation:op.into(),status:BrokerStatus::Accepted,evidence_source:EVIDENCE_SOURCE_INTERNAL_STATE.into(),audit_event_id:event.event_id,error:None,health:None,body:Some(body),shutdown_requested:false}
    }
}
