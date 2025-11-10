use crate::api;
use crate::components::use_app_context;
use crate::models::SessionUser;
use dioxus::logger::tracing::{error, info};
use dioxus::prelude::*;

#[component]
pub fn Login() -> Element {
    let mut qq = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);
    let mut warning = use_signal(|| None::<String>);
    let nav = use_navigator();
    let app_ctx = use_app_context();
    let mut current_user = app_ctx.current_user;

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();

        let qq_val = qq.read().clone();
        let pwd_val = password.read().clone();

        if qq_val.is_empty() || pwd_val.is_empty() {
            error.set(Some("请输入QQ号和密码".to_string()));
            return;
        }

        // 使用API调用替代直接数据库访问
        spawn(async move {
            let req = api::LoginRequest {
                qq: qq_val.clone(),
                password: pwd_val,
            };

            match api::login(req).await {
                Ok(login_resp) => {
                    // 将API返回的用户信息转换为SessionUser
                    let session_user = SessionUser {
                        qq: login_resp.user.qq.clone(),
                        nickname: login_resp.user.nickname.clone(),
                        birthday: login_resp.user.birthday.clone(),
                        main_role_id: None, // API没有返回role_id
                        role_name: login_resp.user.role_name.clone(),
                        permissions: login_resp.user.permissions.clone(),
                    };

                    current_user.set(Some(session_user));
                    success.set(true);
                    error.set(None);
                    info!("用户 {} 登录成功", login_resp.user.nickname);
                    qq.set(String::new());
                    password.set(String::new());

                    // 检查是否使用默认密码
                    if login_resp.user.is_default_password {
                        warning.set(Some(
                            "您正在使用默认密码，为了账号安全，请立即前往个人中心修改密码！"
                                .to_string(),
                        ));
                        // 延迟跳转，让用户看到警告提示
                        spawn(async move {
                            gloo_timers::future::TimeoutFuture::new(3000).await;
                            nav.push(crate::Route::Home {});
                        });
                    } else {
                        // 正常用户立即跳转
                        nav.push(crate::Route::Home {});
                    }
                }
                Err(e) => {
                    error.set(Some(format!("登录失败: {}", e)));
                    error!("登录异常: {e}");
                }
            }
        });
    };

    rsx! {
        div { class: "page-container",
            div { class: "form-container",
                h1 { "用户登录" }

                form { onsubmit: on_submit,
                    div { class: "form-group",
                        label { r#for: "qq", "QQ号：" }
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
                        label { r#for: "password", "密码：" }
                        input {
                            r#type: "password",
                            id: "password",
                            name: "password",
                            placeholder: "请输入密码",
                            value: "{password}",
                            oninput: move |evt| password.set(evt.value().clone())
                        }
                    }

                    if let Some(err) = error.read().as_ref() {
                        div { class: "error-message", "{err}" }
                    }

                    if *success.read() {
                        div { class: "success-message", "登录成功！" }
                    }

                    if let Some(warn) = warning.read().as_ref() {
                        div { class: "warning-message", "{warn}" }
                    }

                    div { class: "form-actions",
                        button { r#type: "submit", class: "btn-primary", "登录" }
                        Link { to: crate::Route::Register {},
                            button { r#type: "button", class: "btn-secondary", "注册账号" }
                        }
                    }
                }
            }
        }
    }
}
