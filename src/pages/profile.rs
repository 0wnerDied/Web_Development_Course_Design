use crate::api;
use crate::components::use_app_context;
use crate::models::SessionUser;
use dioxus::prelude::*;

#[component]
pub fn Profile() -> Element {
    let app_ctx = use_app_context();
    let mut current_user = app_ctx.current_user;

    let mut profile = use_signal(|| None::<api::UserInfo>);
    let mut nickname = use_signal(String::new);
    let mut birthday = use_signal(String::new);
    let mut old_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| None::<String>);
    let mut loading_profile = use_signal(|| false);
    let mut saving_profile = use_signal(|| false);
    let mut saving_password = use_signal(|| false);

    let load_profile = move || {
        spawn(async move {
            loading_profile.set(true);
            match api::get_profile().await {
                Ok(user) => {
                    nickname.set(user.nickname.clone());
                    birthday.set(user.birthday.clone().unwrap_or_default());
                    profile.set(Some(user.clone()));
                    current_user.set(Some(SessionUser {
                        qq: user.qq.clone(),
                        nickname: user.nickname.clone(),
                        birthday: user.birthday.clone(),
                        main_role_id: None,
                        role_name: user.role_name.clone(),
                        permissions: user.permissions.clone(),
                    }));
                    error.set(None);
                    success.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("加载个人信息失败: {}", e)));
                    success.set(None);
                }
            }
            loading_profile.set(false);
        });
    };

    use_effect(move || {
        load_profile();
    });

    let on_save_profile = move |evt: Event<FormData>| {
        evt.prevent_default();

        let nickname_val = nickname.read().trim().to_string();
        if nickname_val.is_empty() {
            error.set(Some("昵称不能为空".to_string()));
            success.set(None);
            return;
        }

        let birthday_val = birthday.read().trim().to_string();

        let mut request = api::UpdateProfileRequest::default();
        request.nickname = Some(nickname_val.clone());
        request.birthday = if birthday_val.is_empty() {
            None
        } else {
            Some(birthday_val.clone())
        };

        saving_profile.set(true);
        success.set(None);
        spawn(async move {
            match api::update_profile(request).await {
                Ok(user) => {
                    nickname.set(user.nickname.clone());
                    birthday.set(user.birthday.clone().unwrap_or_default());
                    profile.set(Some(user.clone()));
                    current_user.set(Some(SessionUser {
                        qq: user.qq.clone(),
                        nickname: user.nickname.clone(),
                        birthday: user.birthday.clone(),
                        main_role_id: None,
                        role_name: user.role_name.clone(),
                        permissions: user.permissions.clone(),
                    }));
                    error.set(None);
                    success.set(Some("个人信息已更新".to_string()));
                }
                Err(e) => {
                    error.set(Some(format!("更新失败: {}", e)));
                }
            }
            saving_profile.set(false);
        });
    };

    let on_change_password = move |evt: Event<FormData>| {
        evt.prevent_default();

        let old_pwd = old_password.read().trim().to_string();
        let new_pwd = new_password.read().trim().to_string();
        let confirm_pwd = confirm_password.read().trim().to_string();

        if old_pwd.is_empty() {
            error.set(Some("请输入原密码".to_string()));
            success.set(None);
            return;
        }

        if new_pwd.len() < 6 {
            error.set(Some("新密码至少需要6位字符".to_string()));
            success.set(None);
            return;
        }

        if new_pwd != confirm_pwd {
            error.set(Some("两次输入的新密码不一致".to_string()));
            success.set(None);
            return;
        }

        saving_password.set(true);
        success.set(None);
        spawn(async move {
            match api::change_password(old_pwd.clone(), new_pwd.clone()).await {
                Ok(msg) => {
                    error.set(None);
                    success.set(Some(msg));
                    old_password.set(String::new());
                    new_password.set(String::new());
                    confirm_password.set(String::new());
                }
                Err(e) => {
                    error.set(Some(format!("修改密码失败: {}", e)));
                }
            }
            saving_password.set(false);
        });
    };

    let profile_snapshot = profile.read().clone();
    let loading_flag = *loading_profile.read();
    let saving_profile_flag = *saving_profile.read();
    let saving_password_flag = *saving_password.read();

    rsx! {
        div { class: "page-container",
            h1 { "个人中心" }

            if let Some(err) = error.read().as_ref() {
                div { class: "error-message", "{err}" }
            }

            if let Some(msg) = success.read().as_ref() {
                div { class: "success-message", "{msg}" }
            }

            if loading_flag && profile_snapshot.is_none() {
                div { class: "loading-message", "正在加载个人信息..." }
            }

            if let Some(user) = profile_snapshot.as_ref() {
                div { class: "profile-card",
                    h2 { "{user.nickname} ({user.qq})" }
                    if let Some(role) = &user.role_name {
                        p { class: "profile-role", "角色: {role}" }
                    }
                    if let Some(birth) = &user.birthday {
                        p { "生日: {birth}" }
                    }
                    if user.permissions.is_empty() {
                        p { "暂无权限信息" }
                    } else {
                        div { class: "profile-permissions",
                            h3 { "拥有的权限" }
                            ul {
                                for perm in user.permissions.iter() {
                                    li { "{perm}" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "profile-layout",
                div { class: "profile-panel",
                    h2 { "基本信息" }
                    form { onsubmit: on_save_profile,
                        div { class: "form-group",
                            label { r#for: "nickname", "昵称" }
                            input {
                                id: "nickname",
                                r#type: "text",
                                value: "{nickname}",
                                oninput: move |evt| nickname.set(evt.value().clone()),
                                disabled: saving_profile_flag || loading_flag,
                            }
                        }
                        div { class: "form-group",
                            label { r#for: "birthday", "生日" }
                            input {
                                id: "birthday",
                                r#type: "date",
                                value: "{birthday}",
                                oninput: move |evt| birthday.set(evt.value().clone()),
                                disabled: saving_profile_flag || loading_flag,
                            }
                        }
                        button {
                            class: "btn-primary",
                            r#type: "submit",
                            disabled: saving_profile_flag,
                            if saving_profile_flag {
                                "保存中..."
                            } else {
                                "保存资料"
                            }
                        }
                    }
                }

                div { class: "profile-panel",
                    h2 { "修改密码" }
                    form { onsubmit: on_change_password,
                        div { class: "form-group",
                            label { r#for: "old_password", "原密码" }
                            input {
                                id: "old_password",
                                r#type: "password",
                                value: "{old_password}",
                                oninput: move |evt| old_password.set(evt.value().clone()),
                                disabled: saving_password_flag,
                            }
                        }
                        div { class: "form-group",
                            label { r#for: "new_password", "新密码" }
                            input {
                                id: "new_password",
                                r#type: "password",
                                value: "{new_password}",
                                oninput: move |evt| new_password.set(evt.value().clone()),
                                disabled: saving_password_flag,
                            }
                        }
                        div { class: "form-group",
                            label { r#for: "confirm_password", "确认新密码" }
                            input {
                                id: "confirm_password",
                                r#type: "password",
                                value: "{confirm_password}",
                                oninput: move |evt| confirm_password.set(evt.value().clone()),
                                disabled: saving_password_flag,
                            }
                        }
                        button {
                            class: "btn-secondary",
                            r#type: "submit",
                            disabled: saving_password_flag,
                            if saving_password_flag {
                                "提交中..."
                            } else {
                                "更新密码"
                            }
                        }
                    }
                }
            }
        }
    }
}
