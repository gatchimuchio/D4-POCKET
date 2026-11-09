//! WindowsでProvider資格情報を入力するnative画面。
//! 秘密値はRustデスクトップ起動器内だけへ返し、Flutterへ渡さない。

use std::{cell::RefCell, rc::Rc};

use winsafe::{co, gui, prelude::*};
use zeroize::{Zeroize, Zeroizing};

const MAX_SECRET_BYTES: usize = 16 * 1024;
const MAX_EDIT_UNITS: u32 = 8 * 1024;

pub(super) fn enter_provider_secret(
    credential_id: &str,
    target: &str,
) -> Option<Zeroizing<String>> {
    if credential_id.len() != 32
        || !credential_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || target != "openai_codex_cli"
    {
        return None;
    }

    let window = gui::WindowMain::new(gui::WindowMainOpts {
        title: "D4 Pocket 資格情報入力",
        size: gui::dpi(520, 220),
        ex_style: co::WS_EX::LEFT | co::WS_EX::DLGMODALFRAME,
        ..Default::default()
    });
    let _details = gui::Label::new(
        &window,
        gui::LabelOpts {
            text: "OpenAI API keyを入力してください。値は画面へ表示されず、Flutter・通常IPC・Auditへ返しません。\n入力後、別のOwner確認を行い、承認された場合だけRust Brokerが現在ユーザーのDPAPIで保管します。",
            position: gui::dpi(18, 14),
            size: gui::dpi(480, 62),
            ..Default::default()
        },
    );
    let edit = gui::Edit::new(
        &window,
        gui::EditOpts {
            position: gui::dpi(18, 82),
            width: gui::dpi_x(480),
            height: gui::dpi_y(28),
            control_style: co::ES::PASSWORD | co::ES::AUTOHSCROLL | co::ES::NOHIDESEL,
            ..Default::default()
        },
    );
    edit.limit_text(Some(MAX_EDIT_UNITS));
    let status = gui::Label::new(
        &window,
        gui::LabelOpts {
            text: "",
            position: gui::dpi(18, 118),
            size: gui::dpi(480, 30),
            ..Default::default()
        },
    );
    let accept = gui::Button::new(
        &window,
        gui::ButtonOpts {
            text: "続行",
            position: gui::dpi(312, 166),
            width: gui::dpi_x(88),
            control_style: co::BS::DEFPUSHBUTTON,
            ..Default::default()
        },
    );
    let cancel = gui::Button::new(
        &window,
        gui::ButtonOpts {
            text: "キャンセル",
            position: gui::dpi(408, 166),
            width: gui::dpi_x(90),
            ..Default::default()
        },
    );

    let entered = Rc::new(RefCell::new(None::<Zeroizing<String>>));
    let edit_for_accept = edit.clone();
    let status_for_accept = status.clone();
    let window_for_accept = window.clone();
    let entered_for_accept = entered.clone();
    accept.on().bn_clicked(move || {
        let Ok(value) = edit_for_accept.text() else {
            let _ =
                status_for_accept.set_text_and_resize("入力を読み取れません。値は保存しません。");
            return Ok(());
        };
        let mut value = Zeroizing::new(value);
        let valid = !value.is_empty()
            && value.len() <= MAX_SECRET_BYTES
            && !value.chars().any(char::is_control);
        let _ = edit_for_accept.set_text("");
        if !valid {
            value.zeroize();
            let _ = status_for_accept
                .set_text_and_resize("空欄・制御文字・上限超過の値は登録できません。");
            return Ok(());
        }
        entered_for_accept.borrow_mut().replace(value);
        window_for_accept.close();
        Ok(())
    });

    let edit_for_cancel = edit.clone();
    let window_for_cancel = window.clone();
    cancel.on().bn_clicked(move || {
        let _ = edit_for_cancel.set_text("");
        window_for_cancel.close();
        Ok(())
    });

    let edit_for_close = edit.clone();
    let window_for_close = window.clone();
    window.on().wm_close(move || {
        let _ = edit_for_close.set_text("");
        window_for_close.hwnd().DestroyWindow()?;
        Ok(())
    });

    let edit_for_focus = edit.clone();
    window.on().wm_create(move |_| {
        let _ = edit_for_focus.hwnd().SetFocus();
        Ok(0)
    });

    window.run_main(None).ok()?;
    let result = entered.borrow_mut().take();
    result
}

#[cfg(test)]
mod tests {
    use super::enter_provider_secret;

    #[test]
    #[ignore = "Windows native UIの限定smoke。合成値だけを入力する"]
    fn provider_secret_dialog_returns_the_entered_value_without_projection() {
        let value = enter_provider_secret("0123456789abcdef0123456789abcdef", "openai_codex_cli")
            .expect("続行操作が合成入力値を返す");
        assert_eq!(&*value, "fixture-not-a-real-provider-key-0123");
    }
}
