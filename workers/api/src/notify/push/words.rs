//! WHAT A PUSH SAYS, in the subscriber's language (sq, en, uk, ru), PURE.
//!
//! SHORT AND ABOUT THE ORDER ONLY: the order's first eight characters and a
//! state. Never a name, a phone, an address or a dish -- a lock screen is
//! read by whoever is holding the phone, and the push service relays it.
//! The status words are the storefront's own (`store/i18n.js` `st`).

/// The order id as people say it (the console and the storefront show the same).
pub fn short(order_id: &str) -> String {
    order_id.chars().take(8).collect()
}

fn pick<'a>(lang: &str, sq: &'a str, en: &'a str, uk: &'a str, ru: &'a str) -> &'a str {
    match lang {
        "sq" => sq,
        "uk" => uk,
        "ru" => ru,
        _ => en,
    }
}

/// The title line: "Order #1a2b3c4d".
pub fn title(lang: &str, order_id: &str) -> String {
    let w = pick(lang, "Porosia", "Order", "Замовлення", "Заказ");
    format!("{w} #{}", short(order_id))
}

/// A status as the customer reads it; `None` for one nobody is told about.
pub fn status(lang: &str, st: &str) -> Option<&'static str> {
    Some(match st {
        "CONFIRMED" => pick(lang, "U konfirmua", "Confirmed", "Підтверджено", "Подтверждён"),
        "PREPARING" => pick(lang, "Po gatuhet", "Being prepared", "Готується", "Готовится"),
        "READY" => pick(lang, "Gati", "Ready", "Готове", "Готов"),
        "IN_DELIVERY" => pick(lang, "Në rrugë", "On the way", "У дорозі", "В пути"),
        "DELIVERED" => pick(lang, "U dorëzua", "Delivered", "Доставлено", "Доставлен"),
        "PICKED_UP" => pick(lang, "U mor", "Collected", "Отримано", "Получен"),
        "REJECTED" => pick(lang, "U refuzua", "Rejected", "Відхилено", "Отклонён"),
        "CANCELLED" => pick(lang, "U anulua", "Cancelled", "Скасовано", "Отменён"),
        "SCHEDULED" => pick(lang, "U planifikua", "Scheduled", "Заплановано", "Запланирован"),
        "REFUNDING" => pick(lang, "Po kthehen paratë", "Refund in progress", "Повернення коштів", "Возврат средств"),
        "COMPENSATED_REFUND" => pick(lang, "Paratë u kthyen", "Refunded", "Кошти повернено", "Средства возвращены"),
        // PENDING is the placement itself: the customer is looking at it.
        _ => return None,
    })
}

/// To the venue's staff: a new order has landed.
pub fn new_order(lang: &str) -> &'static str {
    pick(lang, "Porosi e re", "New order", "Нове замовлення", "Новый заказ")
}

/// To a courier: an order was handed to you.
pub fn assigned(lang: &str) -> &'static str {
    pick(lang, "Ju është caktuar një porosi", "An order was assigned to you", "Вам призначено замовлення", "Вам назначен заказ")
}

/// To the courier of an order: it is ready to collect.
pub fn ready_to_collect(lang: &str) -> &'static str {
    pick(lang, "Gati për t'u marrë", "Ready to collect", "Готове до видачі", "Готов к выдаче")
}

/// Is this a status anybody is told about? The set is the same in every
/// language, so it is asked of the default arm.
pub fn speaks(st: &str) -> bool {
    status("", st).is_some()
}

#[cfg(test)]
mod tests;
