//! PostOrderHooks — відгуки, підписки, хуки на фіналі арки.
//!
//! Після завершення замовлення (етап Review) система пропонує:
//! - Залишити відгук про замовлення
//! - Підписатись на медіа-канали власника (Instagram, Telegram, WhatsApp)
//! - Кожен хук має власний splat-конфіг для анімованої появи
//!
//! Хуки відображаються як невеликі "візитівки" з іконками, які
//! з'являються через splat-анімацію. Ніякого нав'язування — тільки
//! м'яке запрошення (discovery).

/// Канал для підписки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookChannel {
    Instagram,
    Telegram,
    WhatsApp,
}

impl HookChannel {
    pub fn name(&self) -> &'static str {
        match self {
            HookChannel::Instagram => "Instagram",
            HookChannel::Telegram => "Telegram",
            HookChannel::WhatsApp => "WhatsApp",
        }
    }
}

/// Тип хука.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookType {
    Review,
    Subscribe,
}

/// Тип анімації splat-появи.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplatAnim {
    FadeIn,
    Pop,
    SlideUp,
    Pulse,
}

/// Конфігурація splat-іконки для хука.
#[derive(Debug, Clone, PartialEq)]
pub struct IconSplatConfig {
    pub icon: &'static str,
    pub size: f32,
    pub color: [f32; 4],
    pub animation: SplatAnim,
}

/// Один післязамовний хук.
#[derive(Debug, Clone, PartialEq)]
pub struct PostOrderHook {
    pub hook_type: HookType,
    pub channel: HookChannel,
    pub text: &'static str,
    pub icon_config: IconSplatConfig,
    pub action_url: &'static str,
}

impl PostOrderHook {
    /// Хук для відгуку.
    pub fn review() -> Self {
        PostOrderHook {
            hook_type: HookType::Review,
            channel: HookChannel::Telegram,
            text: "Залишити відгук про замовлення",
            icon_config: IconSplatConfig {
                icon: "★",
                size: 32.0,
                color: [0.831, 0.686, 0.216, 1.0],
                animation: SplatAnim::Pop,
            },
            action_url: "/review",
        }
    }

    /// Хук для підписки на Instagram.
    pub fn subscribe_instagram() -> Self {
        PostOrderHook {
            hook_type: HookType::Subscribe,
            channel: HookChannel::Instagram,
            text: "Підписатись на Instagram",
            icon_config: IconSplatConfig {
                icon: "📷",
                size: 28.0,
                color: [0.9, 0.3, 0.5, 1.0],
                animation: SplatAnim::FadeIn,
            },
            action_url: "https://instagram.com/",
        }
    }

    /// Хук для підписки на Telegram.
    pub fn subscribe_telegram() -> Self {
        PostOrderHook {
            hook_type: HookType::Subscribe,
            channel: HookChannel::Telegram,
            text: "Підписатись на Telegram-канал",
            icon_config: IconSplatConfig {
                icon: "✈",
                size: 28.0,
                color: [0.2, 0.6, 0.9, 1.0],
                animation: SplatAnim::SlideUp,
            },
            action_url: "https://t.me/",
        }
    }

    /// Хук для підписки на WhatsApp.
    pub fn subscribe_whatsapp() -> Self {
        PostOrderHook {
            hook_type: HookType::Subscribe,
            channel: HookChannel::WhatsApp,
            text: "Отримувати сповіщення в WhatsApp",
            icon_config: IconSplatConfig {
                icon: "💬",
                size: 28.0,
                color: [0.2, 0.7, 0.3, 1.0],
                animation: SplatAnim::Pulse,
            },
            action_url: "https://wa.me/",
        }
    }

    /// Усі стандартні хуки для показу після замовлення.
    pub fn default_hooks() -> Vec<PostOrderHook> {
        vec![
            PostOrderHook::review(),
            PostOrderHook::subscribe_instagram(),
            PostOrderHook::subscribe_telegram(),
            PostOrderHook::subscribe_whatsapp(),
        ]
    }
}

/// Згенерувати хуки на основі конфігурації власника.
/// channels — які канали підтримує власник (назви: "instagram", "telegram", "whatsapp").
pub fn hooks_for_owner(channels: &[HookChannel]) -> Vec<PostOrderHook> {
    let mut hooks = vec![PostOrderHook::review()];
    for ch in channels {
        match ch {
            HookChannel::Instagram => hooks.push(PostOrderHook::subscribe_instagram()),
            HookChannel::Telegram => hooks.push(PostOrderHook::subscribe_telegram()),
            HookChannel::WhatsApp => hooks.push(PostOrderHook::subscribe_whatsapp()),
        }
    }
    hooks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_hook_has_correct_type() {
        let hook = PostOrderHook::review();
        assert_eq!(hook.hook_type, HookType::Review);
        assert_eq!(hook.channel, HookChannel::Telegram);
    }

    #[test]
    fn subscribe_hooks_have_subscribe_type() {
        for hook in &[PostOrderHook::subscribe_instagram(), PostOrderHook::subscribe_telegram(), PostOrderHook::subscribe_whatsapp()] {
            assert_eq!(hook.hook_type, HookType::Subscribe);
        }
    }

    #[test]
    fn default_hooks_includes_review_and_three_subscriptions() {
        let hooks = PostOrderHook::default_hooks();
        assert_eq!(hooks.len(), 4);
        assert!(hooks.iter().any(|h| h.hook_type == HookType::Review));
        assert!(hooks.iter().any(|h| h.channel == HookChannel::Instagram));
        assert!(hooks.iter().any(|h| h.channel == HookChannel::Telegram));
        assert!(hooks.iter().any(|h| h.channel == HookChannel::WhatsApp));
    }

    #[test]
    fn hooks_for_owner_filters_channels() {
        let hooks = hooks_for_owner(&[HookChannel::Instagram, HookChannel::Telegram]);
        assert_eq!(hooks.len(), 3); // 1 review + 2 subscribe
        assert!(hooks.iter().any(|h| h.hook_type == HookType::Review));
        assert!(hooks.iter().any(|h| h.channel == HookChannel::Instagram));
        assert!(hooks.iter().any(|h| h.channel == HookChannel::Telegram));
        assert!(hooks.iter().all(|h| h.channel != HookChannel::WhatsApp));
    }

    #[test]
    fn all_hooks_have_non_empty_text() {
        for hook in PostOrderHook::default_hooks() {
            assert!(!hook.text.is_empty());
            assert!(!hook.action_url.is_empty());
        }
    }

    #[test]
    fn channel_names_are_displayable() {
        assert_eq!(HookChannel::Instagram.name(), "Instagram");
        assert_eq!(HookChannel::Telegram.name(), "Telegram");
        assert_eq!(HookChannel::WhatsApp.name(), "WhatsApp");
    }
}
