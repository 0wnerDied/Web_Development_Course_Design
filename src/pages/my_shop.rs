use crate::api;
use crate::components::use_current_user;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn MyShop() -> Element {
    let mut my_items = use_signal(Vec::<api::ShopItem>::new);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let loading_visible = use_signal(|| false);
    let mut show_create_form = use_signal(|| false);
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

    // 创建商品表单字段
    let mut item_name = use_signal(String::new);
    let mut item_price = use_signal(String::new);
    let mut item_count = use_signal(|| "1".to_string());
    let mut item_location = use_signal(String::new);

    let mut load_my_items = move || {
        let session = current_user.read().clone();
        let Some(user) = session else {
            error.set(Some("请先登录".to_string()));
            return;
        };

        let seller_qq = user.qq.clone();
        spawn(async move {
            loading.set(true);
            match api::get_shop_items(Some(&seller_qq)).await {
                Ok(item_list) => {
                    my_items.set(item_list);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("加载失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    let create_item = move |evt: Event<FormData>| {
        evt.prevent_default();

        let session = current_user.read().clone();
        let Some(user) = session else {
            error.set(Some("请先登录".to_string()));
            return;
        };

        let name = item_name.read().trim().to_string();
        let price_text = item_price.read().trim().to_string();
        let count_text = item_count.read().trim().to_string();
        let location = item_location.read().trim().to_string();

        if name.is_empty() || price_text.is_empty() || location.is_empty() {
            error.set(Some("请填写所有必填项".to_string()));
            return;
        }

        let price_valid = price_text
            .parse::<f64>()
            .map(|value| value > 0.0)
            .unwrap_or(false);
        if !price_valid {
            error.set(Some("价格必须为大于0的数字".to_string()));
            return;
        }

        let count = count_text.parse::<i32>().unwrap_or(0);
        if count <= 0 {
            error.set(Some("库存必须为正整数".to_string()));
            return;
        }

        let payload = api::CreateItemPayload {
            count,
            price: price_text.clone(),
            name: name.clone(),
            seller: user.qq.clone(),
            location: location.clone(),
        };

        spawn(async move {
            loading.set(true);
            match api::create_shop_item(payload).await {
                Ok(resp) => {
                    success.set(Some(format!("{} (ID: {})", resp.message, resp.id)));
                    error.set(None);
                    // 清空表单
                    item_name.set(String::new());
                    item_price.set(String::new());
                    item_count.set("1".to_string());
                    item_location.set(String::new());
                    show_create_form.set(false);
                    load_my_items();
                }
                Err(e) => {
                    error.set(Some(format!("创建失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load_my_items();
    });

    let user_logged_in = current_user.read().is_some();

    rsx! {
        div { class: "page-container",
            h1 { "我的商品" }

            if !user_logged_in {
                div { class: "info-card",
                    h2 { "需要登录" }
                    p { "请先登录系统，查看和管理您的商品。" }
                    Link { to: crate::Route::Login {},
                        button { class: "btn-primary", "前往登录" }
                    }
                }
            } else {
                div { class: "toolbar",
                    button {
                        class: "btn-primary",
                        onclick: move |_| {
                            let current = *show_create_form.read();
                            show_create_form.set(!current);
                        },
                        disabled: *loading.read(),
                        if *show_create_form.read() { "取消创建" } else { "创建新商品" }
                    }
                    button {
                        class: "btn-secondary",
                        onclick: move |_| load_my_items(),
                        disabled: *loading.read(),
                        "刷新"
                    }
                    Link { to: crate::Route::Shop {},
                        button { class: "btn-secondary", "返回商店" }
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
                        h2 { "创建新商品" }
                        form { onsubmit: create_item,
                            div { class: "form-group",
                                label { "商品名称：*" }
                                input {
                                    r#type: "text",
                                    placeholder: "例如：限量周边",
                                    value: "{item_name}",
                                    oninput: move |evt| item_name.set(evt.value().clone()),
                                    disabled: *loading.read()
                                }
                            }

                            div { class: "form-group",
                                label { "价格（元）：*" }
                                input {
                                    r#type: "text",
                                    placeholder: "所需价格（元），可输入小数",
                                    value: "{item_price}",
                                    oninput: move |evt| item_price.set(evt.value().clone()),
                                    disabled: *loading.read()
                                }
                            }

                            div { class: "form-group",
                                label { "库存数量：*" }
                                input {
                                    r#type: "number",
                                    min: "1",
                                    placeholder: "初始库存",
                                    value: "{item_count}",
                                    oninput: move |evt| item_count.set(evt.value().clone()),
                                    disabled: *loading.read()
                                }
                            }

                            div { class: "form-group",
                                label { "交易地点：*" }
                                input {
                                    r#type: "text",
                                    placeholder: "例如：线上发货 / XX自提点",
                                    value: "{item_location}",
                                    oninput: move |evt| item_location.set(evt.value().clone()),
                                    disabled: *loading.read()
                                }
                            }

                            div { class: "form-actions",
                                button {
                                    r#type: "submit",
                                    class: "btn-primary",
                                    disabled: *loading.read(),
                                    "创建"
                                }
                            }
                        }
                    }
                }

                div { class: "table-container",
                    h2 { "" }
                    if my_items.read().is_empty() {
                        div { class: "empty-state", "暂无商品，点击上方按钮创建" }
                    } else {
                        table { class: "data-table",
                            thead {
                                tr {
                                    th { "ID" }
                                    th { "名称" }
                                    th { "价格" }
                                    th { "库存" }
                                    th { "状态" }
                                    th { "交易地点" }
                                    th { "卖家" }
                                }
                            }
                            tbody {
                                for item in my_items.read().iter() {
                                    tr {
                                        key: "{item.id.unwrap_or_default()}",
                                        td { "{item.id.unwrap_or_default()}" }
                                        td { "{item.name}" }
                                        td { "{item.price} 元" }
                                        td { "{item.count}" }
                                        td {
                                            if item.count > 0 {
                                                span { class: "badge badge-success", "在售" }
                                            } else {
                                                span { class: "badge badge-warning", "已售罄" }
                                            }
                                        }
                                        td { "{item.location}" }
                                        td { "{item.seller}" }
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
