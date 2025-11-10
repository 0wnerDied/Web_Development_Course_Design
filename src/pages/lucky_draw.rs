use crate::api;
use crate::components::use_current_user;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn LuckyDraw() -> Element {
    let mut draws = use_signal(Vec::<api::LuckyDraw>::new);
    let mut shop_items = use_signal(Vec::<api::ShopItem>::new); // 添加商品列表
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let loading_visible = use_signal(|| false);
    let mut show_create_form = use_signal(|| false);

    // 创建抽奖表单字段
    let mut item_id_input = use_signal(String::new);
    let mut fitting = use_signal(String::new);
    let mut num_input = use_signal(|| "1".to_string());
    let mut min_lp_input = use_signal(|| "0".to_string());
    let mut plan_time_input = use_signal(String::new);
    let mut description = use_signal(String::new);

    let current_user = use_current_user();

    {
        let loading = loading;
        let mut loading_visible = loading_visible;
        use_effect(move || {
            if *loading.read() {
                let loading = loading;
                let mut loading_visible = loading_visible;
                spawn(async move {
                    TimeoutFuture::new(180).await;
                    if *loading.read() {
                        loading_visible.set(true);
                    }
                });
            } else {
                loading_visible.set(false);
            }
        });
    }

    let load_draws = move || {
        spawn(async move {
            loading.set(true);
            match api::get_lucky_draws().await {
                Ok(draw_list) => {
                    draws.set(draw_list);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("加载失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    let load_shop_items = move || {
        let session_user = current_user.read().clone();
        spawn(async move {
            // 只加载当前用户自己的商品（只能抽自己的商品）
            if let Some(user) = session_user {
                match api::get_shop_items(Some(&user.qq)).await {
                    Ok(items) => {
                        shop_items.set(items);
                    }
                    Err(_e) => {
                        // 加载商品失败不影响主流程，只是下拉框没有选项
                        shop_items.set(Vec::new());
                    }
                }
            } else {
                // 未登录，不加载商品
                shop_items.set(Vec::new());
            }
        });
    };

    let execute_draw = move |id: i64| {
        spawn(async move {
            loading.set(true);
            match api::execute_lucky_draw(id).await {
                Ok(result) => {
                    let message = if let Some(winner) = result.winner {
                        format!("{} 中奖者: {}", result.message, winner)
                    } else {
                        result.message
                    };
                    success.set(Some(message));
                    error.set(None);
                    load_draws();
                }
                Err(e) => {
                    error.set(Some(format!("抽奖失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load_draws();
        load_shop_items(); // 同时加载商品列表
    });

    let delete_draw = move |draw_id: i64| {
        spawn(async move {
            // 使用 web_sys 的 confirm
            let window = web_sys::window().expect("no global `window` exists");
            let result =
                window.confirm_with_message("确定要删除此抽奖吗？如果未开奖，库存将会被恢复。");

            if let Ok(true) = result {
                loading.set(true);
                match api::delete_lucky_draw(draw_id).await {
                    Ok(msg) => {
                        success.set(Some(msg));
                        error.set(None);
                        load_draws();
                    }
                    Err(e) => {
                        error.set(Some(format!("删除失败: {}", e)));
                    }
                }
                loading.set(false);
            }
        });
    };

    let create_draw = move |evt: Event<FormData>| {
        evt.prevent_default();

        let session_user = current_user.read().clone();
        let Some(user) = session_user else {
            error.set(Some("请先登录后再发起抽奖".to_string()));
            return;
        };

        let num = num_input
            .read()
            .parse::<i32>()
            .map_err(|_| "数量必须为整数".to_string());
        let min_lp = min_lp_input
            .read()
            .parse::<i32>()
            .map_err(|_| "最低LP要求必须为整数".to_string());

        let plan_raw = plan_time_input.read().clone();
        if plan_raw.is_empty() {
            error.set(Some("请选择计划开奖时间".to_string()));
            return;
        }

        let plan_time = match parse_datetime_local(&plan_raw) {
            Ok(dt) => dt,
            Err(msg) => {
                error.set(Some(msg));
                return;
            }
        };

        let num = match num {
            Ok(value) if value > 0 => value,
            Ok(_) => {
                error.set(Some("数量必须大于0".to_string()));
                return;
            }
            Err(msg) => {
                error.set(Some(msg));
                return;
            }
        };

        let min_lp = match min_lp {
            Ok(value) if value >= 0 => value,
            Ok(_) => {
                error.set(Some("最低LP要求不能为负数".to_string()));
                return;
            }
            Err(msg) => {
                error.set(Some(msg));
                return;
            }
        };

        let item_id = {
            let text = item_id_input.read();
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                match trimmed.parse::<i64>() {
                    Ok(id) => Some(id),
                    Err(_) => {
                        error.set(Some("物品ID必须为整数".to_string()));
                        return;
                    }
                }
            }
        };

        let fitting_value = fitting.read().clone();
        let description_value = description.read().clone();

        let payload = api::CreateDrawPayload {
            create_qq: user.qq.clone(),
            item_id,
            fitting: optional_trim(fitting_value),
            num,
            min_lp_require: min_lp,
            plan_time,
            description: optional_trim(description_value),
        };

        spawn(async move {
            loading.set(true);
            match api::create_lucky_draw(payload).await {
                Ok(resp) => {
                    success.set(Some(format!("{} (ID: {})", resp.message, resp.id)));
                    error.set(None);
                    // 清空表单
                    item_id_input.set(String::new());
                    fitting.set(String::new());
                    num_input.set("1".to_string());
                    min_lp_input.set("0".to_string());
                    plan_time_input.set(String::new());
                    description.set(String::new());
                    show_create_form.set(false);
                    load_draws();
                }
                Err(e) => {
                    error.set(Some(format!("创建失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    rsx! {
        div { class: "page-container",
            h1 { "抽奖活动管理" }

            div { class: "toolbar",
                button {
                    class: "btn-primary",
                    onclick: move |_| {
                        let current = *show_create_form.read();
                        show_create_form.set(!current);
                        if !current {
                            // 清空表单
                            item_id_input.set(String::new());
                            fitting.set(String::new());
                            num_input.set("1".to_string());
                            min_lp_input.set("0".to_string());
                            plan_time_input.set(String::new());
                            description.set(String::new());
                        }
                    },
                    disabled: *loading.read(),
                    if *show_create_form.read() { "取消创建" } else { "创建抽奖" }
                }
                button {
                    class: "btn-secondary",
                    onclick: move |_| load_draws(),
                    disabled: *loading.read(),
                    "刷新"
                }
            }

            if *loading_visible.read() {
                div { class: "loading-message", "处理中..." }
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "error-message", "{err}" }
            }

            if let Some(succ) = success.read().as_ref() {
                div { class: "success-message", "{succ}" }
            }

            if *show_create_form.read() {
                div { class: "form-container",
                    h2 { "发起抽奖" }
                    form { onsubmit: create_draw,
                        div { class: "form-group",
                            label { "关联商城商品（可选）：" }
                            select {
                                value: "{item_id_input}",
                                onchange: move |evt| item_id_input.set(evt.value().clone()),
                                disabled: *loading.read(),
                                option { value: "", "不关联商品（自定义奖品）" }
                                for item in shop_items.read().iter() {
                                    option {
                                        value: "{item.id.unwrap_or(0)}",
                                        "{item.name} (库存: {item.count}, 价格: {item.price} 元) - 卖家: {item.seller}"
                                    }
                                }
                            }
                            p {
                                style: "font-size: 13px; color: #666; margin-top: 0.5rem;",
                                "提示：只能抽取您自己发布的商品。选择商品后，抽奖奖品将与该商品关联。如果商品被删除，此字段会自动清空。"
                            }
                        }

                        div { class: "form-group",
                            label { "规格 / 配件说明：" }
                            input {
                                r#type: "text",
                                placeholder: "例如：颜色、尺寸等",
                                value: "{fitting}",
                                oninput: move |evt| fitting.set(evt.value().clone()),
                                disabled: *loading.read()
                            }
                        }

                        div { class: "form-group",
                            label { "奖品数量：*" }
                            input {
                                r#type: "number",
                                min: "1",
                                value: "{num_input}",
                                oninput: move |evt| num_input.set(evt.value().clone()),
                                disabled: *loading.read()
                            }
                        }

                        div { class: "form-group",
                            label { "最低LP要求：*" }
                            input {
                                r#type: "number",
                                min: "0",
                                value: "{min_lp_input}",
                                oninput: move |evt| min_lp_input.set(evt.value().clone()),
                                disabled: *loading.read()
                            }
                        }

                        div { class: "form-group",
                            label { "计划开奖时间：*" }
                            input {
                                r#type: "datetime-local",
                                value: "{plan_time_input}",
                                oninput: move |evt| plan_time_input.set(evt.value().clone()),
                                disabled: *loading.read()
                            }
                        }

                        div { class: "form-group",
                            label { "活动描述：" }
                            textarea {
                                rows: "3",
                                placeholder: "可填写奖品说明、参与方式等",
                                value: "{description}",
                                oninput: move |evt| description.set(evt.value().clone()),
                                disabled: *loading.read()
                            }
                        }

                        div { class: "form-actions",
                            button {
                                r#type: "submit",
                                class: "btn-primary",
                                disabled: *loading.read(),
                                "提交"
                            }
                        }
                    }
                }
            }

            div { class: "table-container",
                table { class: "data-table",
                    thead {
                        tr {
                            th { "ID" }
                            th { "创建时间" }
                            th { "创建人" }
                            th { "奖品信息" }
                            th { "数量" }
                            th { "最低LP" }
                            th { "计划时间" }
                            th { "状态" }
                            th { "中奖者" }
                            th { "描述" }
                            th { "操作" }
                        }
                    }
                    tbody {
                        for draw in draws.read().iter() {
                            tr {
                                key: "{draw.id.unwrap_or(0)}",
                                td { "{draw.id.unwrap_or(0)}" }
                                td { "{draw.create_time}" }
                                td { "{draw.create_qq}" }
                                td {
                                    if let Some(item_id) = draw.item_id {
                                        span {
                                            style: "color: #2196f3; font-weight: 500;",
                                            "商品#{item_id}"
                                        }
                                        if let Some(fitting) = draw.fitting.as_ref() {
                                            br {}
                                            span {
                                                style: "font-size: 12px; color: #666;",
                                                "({fitting})"
                                            }
                                        }
                                    } else if let Some(fitting) = draw.fitting.as_ref() {
                                        "{fitting}"
                                    } else {
                                        "-"
                                    }
                                }
                                td { "{draw.num}" }
                                td { "{draw.min_lp_require}" }
                                td { "{draw.plan_time}" }
                                td {
                                    match draw.status {
                                        0 => rsx!(span { class: "badge badge-warning", "未开奖" }),
                                        1 => rsx!(span { class: "badge badge-success", "已开奖" }),
                                        _ => rsx!(span { class: "badge", "未知" }),
                                    }
                                }
                                td {
                                    if let Some(winner) = draw.winner_qq.as_ref() {
                                        "{winner}"
                                    } else {
                                        "-"
                                    }
                                }
                                td {
                                    if let Some(desc) = draw.description.as_ref() {
                                        "{desc}"
                                    } else {
                                        "-"
                                    }
                                }
                                td {
                                    div {
                                        style: "display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap;",
                                        if draw.status == 0 {
                                            if draw.id.is_some() {
                                                button {
                                                    class: "btn-small btn-success",
                                                    onclick: {
                                                        let draw_id = draw.id;
                                                        move |_| {
                                                            if let Some(id) = draw_id {
                                                                execute_draw(id);
                                                            }
                                                        }
                                                    },
                                                    disabled: *loading.read(),
                                                    "开奖"
                                                }
                                                button {
                                                    class: "btn-small btn-danger",
                                                    onclick: {
                                                        let draw_id = draw.id;
                                                        move |_| {
                                                            if let Some(id) = draw_id {
                                                                delete_draw(id);
                                                            }
                                                        }
                                                    },
                                                    disabled: *loading.read(),
                                                    "删除"
                                                }
                                            } else {
                                                span { "-" }
                                            }
                                        } else {
                                            span { style: "color: #999; font-size: 13px;", "已开奖" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn parse_datetime_local(input: &str) -> Result<String, String> {
    if let Some((date, time)) = input.split_once('T') {
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| "日期格式不正确".to_string())?;
        let time =
            NaiveTime::parse_from_str(time, "%H:%M").map_err(|_| "时间格式不正确".to_string())?;
        let dt = NaiveDateTime::new(date, time);
        Ok(dt.format("%Y-%m-%d %H:%M:00").to_string())
    } else {
        Err("请选择正确的日期和时间".to_string())
    }
}

fn optional_trim(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
