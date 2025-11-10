use crate::api;
use crate::components::use_current_user;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn Shop() -> Element {
    let mut items = use_signal(Vec::<api::ShopItem>::new);
    let mut search_keyword = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let loading_visible = use_signal(|| false);
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

    // 自动清除成功消息
    {
        let success = success;
        use_effect(move || {
            if success.read().is_some() {
                let mut success = success;
                spawn(async move {
                    TimeoutFuture::new(3000).await;
                    success.set(None);
                });
            }
        });
    }

    // 自动清除错误消息
    {
        let error = error;
        use_effect(move || {
            if error.read().is_some() {
                let mut error = error;
                spawn(async move {
                    TimeoutFuture::new(4000).await;
                    error.set(None);
                });
            }
        });
    }

    // 搜索商品
    let search_items = move || {
        let keyword = search_keyword.read().clone();
        let keyword_lower = keyword.trim().to_lowercase();

        spawn(async move {
            loading.set(true);
            success.set(None);
            match api::get_shop_items(None).await {
                Ok(item_list) => {
                    let filtered = filter_shop_items(item_list, &keyword_lower);
                    let count = filtered.len();
                    items.set(filtered);
                    error.set(None);
                    if keyword_lower.is_empty() {
                        success.set(None);
                    } else {
                        success.set(Some(format!("共找到 {} 件匹配商品", count)));
                    }
                }
                Err(e) => {
                    error.set(Some(format!("加载失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // 初始加载所有商品
    let load_all_items = move || {
        spawn(async move {
            loading.set(true);
            match api::get_shop_items(None).await {
                Ok(item_list) => {
                    items.set(item_list);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("加载失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    let mut purchase = move |item_id: i64| {
        let session = current_user.read().clone();
        let Some(user) = session else {
            error.set(Some("请先登录后再购买商品".to_string()));
            return;
        };

        let buyer = user.qq.clone();
        let keyword_lower = search_keyword.read().clone().trim().to_lowercase();

        spawn(async move {
            loading.set(true);
            match api::purchase_shop_item(buyer, item_id, 1).await {
                Ok(message) => {
                    success.set(Some(message));
                    error.set(None);
                    // 购买后重新加载,保持当前的搜索状态
                    if keyword_lower.is_empty() {
                        match api::get_shop_items(None).await {
                            Ok(item_list) => {
                                items.set(item_list);
                            }
                            Err(e) => {
                                error.set(Some(format!("刷新商品失败: {}", e)));
                            }
                        }
                    } else {
                        match api::get_shop_items(None).await {
                            Ok(item_list) => {
                                let filtered = filter_shop_items(item_list, &keyword_lower);
                                items.set(filtered);
                            }
                            Err(e) => {
                                error.set(Some(format!("刷新商品失败: {}", e)));
                            }
                        }
                    }
                }
                Err(e) => {
                    error.set(Some(format!("购买失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // 初始加载
    use_effect(move || {
        load_all_items();
    });

    rsx! {
        div { class: "page-container",
            h1 {
                style: "display: flex; align-items: center; gap: 0.75rem;",
                "虚拟商店（仅作为交易凭据）"
            }

            div { class: "toolbar",
                div { class: "search-box",
                    input {
                        r#type: "text",
                        placeholder: "搜索商品名称、地点或卖家...",
                        value: "{search_keyword}",
                        oninput: move |evt| search_keyword.set(evt.value().clone()),
                        disabled: *loading.read()
                    }
                    button {
                        class: "btn-primary",
                        onclick: move |_| search_items(),
                        disabled: *loading.read(),
                        "搜索"
                    }
                }

                Link { to: crate::Route::MyShop {},
                    button {
                        class: "btn-secondary",
                        style: "display: inline-flex; align-items: center; gap: 0.5rem;",
                        "我的商品"
                    }
                }
                Link { to: crate::Route::ShopTransactions {},
                    button {
                        class: "btn-secondary",
                        style: "display: inline-flex; align-items: center; gap: 0.5rem;",
                        "交易记录"
                    }
                }
                button {
                    class: "btn-secondary",
                    onclick: move |_| load_all_items(),
                    disabled: *loading.read(),
                    "刷新"
                }
            }

            if *loading_visible.read() {
                div { class: "loading-message", "加载中..." }
            }

            // Toast 通知容器
            div { class: "toast-container",
                if let Some(err) = error.read().as_ref() {
                    div { class: "toast toast-error", "{err}" }
                }

                if let Some(succ) = success.read().as_ref() {
                    div { class: "toast toast-success", "{succ}" }
                }
            }

            if items.read().is_empty() {
                div {
                    style: "display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 6rem 2rem; color: #999;",
                    div { style: "font-size: 20px; font-weight: 600; margin-bottom: 0.5rem;", "暂无商品" }
                    div { style: "font-size: 14px; color: #bbb;", "稍后再来看看吧~" }
                }
            } else {
                div { class: "shop-grid",
                    for item in items.read().iter() {
                        div { class: "shop-item-card",
                            key: "{item.id.unwrap_or_default()}",
                            h3 { "{item.name}" }
                            p { class: "price", "价格: {item.price} 元" }
                            p { "库存: {item.count} 件" }
                            p { "交易地点: {item.location}" }
                            p { class: "seller", "卖家: {item.seller}" }
                            p { class: "status",
                                if item.count > 0 {
                                    span { class: "badge badge-success", "可购买" }
                                } else {
                                    span { class: "badge badge-warning", "已售罄" }
                                }
                            }
                            button {
                                class: "btn-primary",
                                style: if item.count > 0 { "" } else { "opacity: 0.5; cursor: not-allowed;" },
                                onclick: {
                                    let id = item.id;
                                    move |_| {
                                        if let Some(actual) = id {
                                            purchase(actual);
                                        }
                                    }
                                },
                                disabled: *loading.read() || item.id.is_none() || item.count <= 0,
                                if item.count > 0 {
                                    "立即购买"
                                } else {
                                    "已售罄"
                                }
                            }
                        }
                    }
                }

                div {
                    class: "stats",
                    style: "display: flex; align-items: center; justify-content: center; gap: 0.5rem;",
                    "共 {items.read().len()} 件商品"
                }
            }
        }
    }
}

fn filter_shop_items(items: Vec<api::ShopItem>, keyword: &str) -> Vec<api::ShopItem> {
    if keyword.is_empty() {
        return items;
    }

    items
        .into_iter()
        .filter(|item| {
            let name = item.name.to_lowercase();
            let location = item.location.to_lowercase();
            let seller = item.seller.to_lowercase();
            name.contains(keyword) || location.contains(keyword) || seller.contains(keyword)
        })
        .collect()
}
