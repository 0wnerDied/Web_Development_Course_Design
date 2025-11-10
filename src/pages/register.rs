use crate::api;
use dioxus::prelude::*;

#[component]
pub fn Register() -> Element {
    let mut qq = use_signal(String::new);
    let mut nickname = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut birthday = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();

        let qq_val = qq.read().clone();
        let nickname_val = nickname.read().clone();
        let pwd_val = password.read().clone();
        let confirm_pwd = confirm_password.read().clone();
        let birthday_val = birthday.read().clone();

        // 验证
        if qq_val.is_empty() || nickname_val.is_empty() || pwd_val.is_empty() {
            error.set(Some("请填写所有必填项".to_string()));
            return;
        }

        if pwd_val != confirm_pwd {
            error.set(Some("两次输入的密码不一致".to_string()));
            return;
        }

        if pwd_val.len() < 6 {
            error.set(Some("密码长度至少6位".to_string()));
            return;
        }

        let birthday_opt = if birthday_val.is_empty() {
            None
        } else {
            Some(birthday_val)
        };

        // 使用API调用替代直接数据库访问
        spawn(async move {
            let req = api::RegisterRequest {
                qq: qq_val,
                nickname: nickname_val,
                password: pwd_val,
                birthday: birthday_opt,
            };

            match api::register(req).await {
                Ok(_) => {
                    success.set(true);
                    error.set(None);
                    qq.set(String::new());
                    nickname.set(String::new());
                    password.set(String::new());
                    confirm_password.set(String::new());
                    birthday.set(String::new());
                }
                Err(e) => {
                    error.set(Some(format!("注册失败: {}", e)));
                }
            }
        });
    };

    rsx! {
        div { class: "page-container",
            div { class: "form-container",
                h1 { "用户注册" }

                form { onsubmit: on_submit,
                    div { class: "form-group",
                        label { r#for: "qq", "QQ号：*" }
                        input {
                            r#type: "text",
                            id: "qq",
                            name: "qq",
                            placeholder: "请输入QQ号",
                            value: "{qq}",
                            oninput: move |evt| qq.set(evt.value().clone())
                        }
                    }

                    div { class: "form-group",
                        label { r#for: "nickname", "昵称：*" }
                        input {
                            r#type: "text",
                            id: "nickname",
                            name: "nickname",
                            placeholder: "请输入昵称",
                            value: "{nickname}",
                            oninput: move |evt| nickname.set(evt.value().clone())
                        }
                    }

                    div { class: "form-group",
                        label { r#for: "password", "密码：*" }
                        input {
                            r#type: "password",
                            id: "password",
                            name: "password",
                            placeholder: "请输入密码（至少6位）",
                            value: "{password}",
                            oninput: move |evt| password.set(evt.value().clone())
                        }
                    }

                    div { class: "form-group",
                        label { r#for: "confirm_password", "确认密码：*" }
                        input {
                            r#type: "password",
                            id: "confirm_password",
                            name: "confirm_password",
                            placeholder: "请再次输入密码",
                            value: "{confirm_password}",
                            oninput: move |evt| confirm_password.set(evt.value().clone())
                        }
                    }

                    div { class: "form-group",
                        label { r#for: "birthday", "生日：" }
                        input {
                            r#type: "date",
                            id: "birthday",
                            name: "birthday",
                            value: "{birthday}",
                            oninput: move |evt| birthday.set(evt.value().clone())
                        }
                    }

                    if let Some(err) = error.read().as_ref() {
                        div { class: "error-message", "{err}" }
                    }

                    if *success.read() {
                        div { class: "success-message",
                            "注册成功！"
                            Link { to: crate::Route::Login {}, " 立即登录" }
                        }
                    }

                    div { class: "form-actions",
                        button { r#type: "submit", class: "btn-primary", "注册" }
                        Link { to: crate::Route::Login {},
                            button { r#type: "button", class: "btn-secondary", "返回登录" }
                        }
                    }
                }
            }
        }
    }
}
