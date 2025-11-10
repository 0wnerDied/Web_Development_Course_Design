use crate::api;
use crate::components::use_current_user;
use dioxus::prelude::*;

#[component]
pub fn LpSubmit() -> Element {
    let mut user_qq = use_signal(String::new);
    let mut lp_type_id = use_signal(|| None::<i64>);
    let mut lp_num = use_signal(|| 1i32);
    let mut reason = use_signal(String::new);
    let mut role = use_signal(String::new);
    let mut picture = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut success_message = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let current_user = use_current_user();
    let mut lp_types = use_signal(Vec::<api::LpType>::new);

    // 加载LP类型（只在组件挂载时执行一次）
    use_hook(|| {
        spawn(async move {
            match api::get_lp_types().await {
                Ok(types) => {
                    if !types.is_empty() {
                        lp_types.set(types);
                        let first_valid = lp_types.read().iter().find_map(|tp| tp.id);
                        lp_type_id.set(first_valid);
                    }
                }
                Err(e) => {
                    error.set(Some(format!("加载LP类型失败: {}", e)));
                }
            }
        });
    });

    // 获取当前选中的LP类型名称（用于实时验证）
    let current_lp_type_name = use_memo(move || {
        let types = lp_types.read();
        let selected = lp_type_id.read();
        types
            .iter()
            .find(|tp| tp.id == *selected)
            .map(|tp| tp.name.clone())
            .unwrap_or_default()
    });

    let on_submit = move |evt: Event<FormData>| {
        evt.prevent_default();

        let session_user = current_user.read().clone();
        if session_user.is_none() {
            error.set(Some("请先登录后再提交LP申请".to_string()));
            return;
        }

        let user_qq_val = user_qq.read().trim().to_string();
        let reason_val = reason.read().trim().to_string();
        let selected_type = *lp_type_id.read();
        let lp_num_val = *lp_num.read();
        let role_val = role.read().trim().to_string();
        let picture_val = picture.read().trim().to_string();

        if user_qq_val.trim().is_empty() || reason_val.trim().is_empty() {
            error.set(Some("请填写所有必填项".to_string()));
            return;
        }

        if selected_type.is_none() {
            error.set(Some("当前缺少可用的LP类型，请联系管理员配置".to_string()));
            return;
        }

        if lp_num_val == 0 {
            error.set(Some("数量不能为0".to_string()));
            return;
        }

        // 根据LP类型验证数量范围
        let lp_types_read = lp_types.read();
        let lp_type_name = lp_types_read
            .iter()
            .find(|tp| tp.id == selected_type)
            .map(|tp| tp.name.as_str())
            .unwrap_or("");

        match lp_type_name {
            "惩罚" => {
                if lp_num_val > 0 {
                    error.set(Some("惩罚类型只能输入小于等于0的值".to_string()));
                    return;
                }
            }
            "奖励" | "兑换" => {
                if lp_num_val < 0 {
                    error.set(Some("奖励和兑换类型只能输入大于等于0的值".to_string()));
                    return;
                }
            }
            "调整" => {
                // 调整类型可以是任意值，不做限制
            }
            _ => {}
        }

        // 使用API提交LP
        spawn(async move {
            loading.set(true);
            let current_user = session_user;
            let submitter = current_user
                .as_ref()
                .map(|u| u.qq.clone())
                .unwrap_or_default();
            match api::submit_lp(
                submitter,
                user_qq_val.clone(),
                selected_type.unwrap(),
                lp_num_val,
                reason_val,
                if picture_val.is_empty() {
                    None
                } else {
                    Some(picture_val.clone())
                },
                if role_val.is_empty() {
                    None
                } else {
                    Some(role_val.clone())
                },
            )
            .await
            {
                Ok(resp) => {
                    success_message.set(Some(resp.message));
                    error.set(None);
                    // 清空表单
                    user_qq.set(String::new());
                    reason.set(String::new());
                    role.set(String::new());
                    picture.set(String::new());
                    lp_num.set(1);
                }
                Err(e) => {
                    error.set(Some(format!("提交失败: {}", e)));
                    success_message.set(None);
                }
            }
            loading.set(false);
        });
    };

    let current_user_snapshot = current_user.read().clone();
    let logged_in = current_user_snapshot.is_some();
    let selected_type_value = lp_type_id
        .read()
        .as_ref()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "".to_string());
    let (submitter_name, submitter_qq) = if let Some(user) = current_user_snapshot.as_ref() {
        (user.nickname.clone(), user.qq.clone())
    } else {
        (String::new(), String::new())
    };

    rsx! {
        div { class: "page-container",
            if !logged_in {
                div { class: "info-card",
                    h2 { "需要登录" }
                    p { "请先登录系统，再提交LP申请。" }
                    Link { to: crate::Route::Login {},
                        button { class: "btn-primary", "前往登录" }
                    }
                }
            } else {
                div { class: "form-container",
                    h1 { "提交LP申请" }
                    p { class: "form-tip", "将由 {submitter_name} (QQ: {submitter_qq}) 提交此申请" }

                    form { onsubmit: on_submit,
                        div { class: "form-group",
                            label { "关联用户QQ：*" }
                            input {
                                r#type: "text",
                                placeholder: "请输入用户QQ号",
                                value: "{user_qq}",
                                oninput: move |evt| user_qq.set(evt.value().clone())
                            }
                        }

                        div { class: "form-group",
                            label { "LP类型：*" }
                            select {
                                value: selected_type_value,
                                onchange: move |evt| {
                                    if let Ok(parsed) = evt.value().parse::<i64>() {
                                        lp_type_id.set(Some(parsed));
                                    }
                                },
                                disabled: *loading.read(),
                                for tp in lp_types.read().iter() {
                                    if let Some(id) = tp.id {
                                        option { value: "{id}", "{tp.name}" }
                                    }
                                }
                            }
                        }

                        div { class: "form-group",
                            label { "数量 (正数加分/负数扣分)：*" }
                            input {
                                r#type: "number",
                                value: "{lp_num}",
                                oninput: move |evt| {
                                    if let Ok(parsed) = evt.value().parse::<i32>() {
                                        lp_num.set(parsed);

                                        // 实时验证数量范围
                                        let type_name = current_lp_type_name.read();
                                        let validation_error = match type_name.as_str() {
                                            "惩罚" if parsed > 0 => Some("惩罚类型只能输入小于等于0的值".to_string()),
                                            "奖励" | "兑换" if parsed < 0 => Some("奖励和兑换类型只能输入大于等于0的值".to_string()),
                                            _ if parsed == 0 => Some("数量不能为0".to_string()),
                                            _ => None,
                                        };

                                        if validation_error.is_some() {
                                            error.set(validation_error);
                                        } else {
                                            // 清除之前的数量验证错误（但保留其他错误）
                                            let should_clear = {
                                                let err_opt = error.read();
                                                err_opt.as_ref().is_some_and(|err_msg| {
                                                    err_msg.contains("只能输入") || err_msg.contains("数量不能为0")
                                                })
                                            };
                                            if should_clear {
                                                error.set(None);
                                            }
                                        }
                                    }
                                },
                                disabled: *loading.read()
                            }
                        }

                        div { class: "form-group",
                            label { "原因说明：*" }
                            textarea {
                                placeholder: "请详细说明原因...",
                                rows: "4",
                                value: "{reason}",
                                oninput: move |evt| reason.set(evt.value().clone()),
                                disabled: *loading.read()
                            }
                        }

                        div { class: "form-group",
                            label { "角色备注：" }
                            input {
                                r#type: "text",
                                placeholder: "例如：团队角色或等级信息",
                                value: "{role}",
                                oninput: move |evt| role.set(evt.value().clone()),
                                disabled: *loading.read()
                            }
                        }

                        div { class: "form-group",
                            label { "凭证图片链接：" }
                            input {
                                r#type: "text",
                                placeholder: "可选，提供图片URL",
                                value: "{picture}",
                                oninput: move |evt| picture.set(evt.value().clone()),
                                disabled: *loading.read()
                            }
                        }

                        if let Some(err) = error.read().as_ref() {
                            div { class: "error-message", "{err}" }
                        }

                        if let Some(msg) = success_message.read().as_ref() {
                            div { class: "success-message", "{msg}" }
                        }

                        if *loading.read() {
                            div { class: "loading-message", "提交中..." }
                        }

                        div { class: "form-actions",
                            button {
                                r#type: "submit",
                                class: "btn-primary",
                                disabled: *loading.read(),
                                "提交申请"
                            }
                            Link { to: crate::Route::LpManagement {},
                                button { r#type: "button", class: "btn-secondary", "返回" }
                            }
                        }
                    }
                }
            }
        }
    }
}
