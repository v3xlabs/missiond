pub mod level;
pub mod notification;
pub mod sidebar;
pub mod sidebar_mode;
pub mod surface;
pub mod surfaces;
pub mod toast;

pub use level::Level;
pub use notification::Notification;
pub use sidebar::Sidebar;
pub use sidebar_mode::SidebarMode;
pub use surface::Surface;
pub use surfaces::Surfaces;
pub use toast::Toast;

use std::time::{Duration, Instant};

use tokio::sync::{watch, Mutex};

use crate::config::NotificationMode;

pub struct Notifications {
    active: Mutex<Vec<Held>>,
    next: Mutex<u64>,
    changed: watch::Sender<u64>,
}

struct Held {
    notification: Notification,
    expires_at: Instant,
}

impl Default for Notifications {
    fn default() -> Self {
        Self::new()
    }
}

impl Notifications {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(Vec::new()),
            next: Mutex::new(1),
            changed: watch::channel(0).0,
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }

    pub async fn push(&self, mut notification: Notification, lifetime: Duration) -> u64 {
        let expires_at = Instant::now() + lifetime;

        // A repeat under the same key keeps its id: the page uses the id as its list key, and a
        // new one remounts the row on every poll.
        if let Some(key) = notification.key.clone() {
            let mut active = self.active.lock().await;

            if let Some(held) = active
                .iter_mut()
                .find(|held| held.notification.key.as_deref() == Some(key.as_str()))
            {
                notification.notification_id = held.notification.notification_id;
                held.notification = notification;
                held.expires_at = expires_at;

                let notification_id = held.notification.notification_id;

                drop(active);
                self.changed.send_modify(|version| *version += 1);

                return notification_id;
            }
        }

        let notification_id = {
            let mut next = self.next.lock().await;
            let id = *next;

            *next += 1;

            id
        };

        notification.notification_id = notification_id;

        self.active.lock().await.push(Held {
            notification,
            expires_at,
        });

        self.changed.send_modify(|version| *version += 1);

        notification_id
    }

    pub async fn dismiss(&self, notification_id: u64) -> bool {
        self.retain(|held| held.notification.notification_id != notification_id)
            .await
    }

    /// Drops every keyed notification the predicate does not name. A notification with no key is
    /// nobody's to retire and is always kept.
    pub async fn retain_keyed(&self, keep: impl Fn(&str) -> bool) -> bool {
        self.retain(|held| match held.notification.key.as_deref() {
            Some(key) => keep(key),
            None => true,
        })
        .await
    }

    async fn retain(&self, keep: impl Fn(&Held) -> bool) -> bool {
        let mut active = self.active.lock().await;
        let before = active.len();

        active.retain(keep);

        let removed = active.len() != before;

        drop(active);

        if removed {
            self.changed.send_modify(|version| *version += 1);
        }

        removed
    }

    /// Expired entries are pruned here and nowhere else, so a caller is what makes an expiry
    /// observable.
    pub async fn active(&self) -> Vec<Notification> {
        let now = Instant::now();
        let mut active = self.active.lock().await;
        let before = active.len();

        active.retain(|held| held.expires_at > now);

        let expired = active.len() != before;
        let notifications = active
            .iter()
            .map(|held| Notification {
                expires_in_seconds: held.expires_at.saturating_duration_since(now).as_secs(),
                ..held.notification.clone()
            })
            .collect();

        drop(active);

        if expired {
            self.changed.send_modify(|version| *version += 1);
        }

        notifications
    }

    pub async fn current(&self) -> Option<Notification> {
        self.active().await.pop()
    }

    pub async fn current_in(&self, mode: NotificationMode) -> Option<Notification> {
        self.active()
            .await
            .into_iter()
            .rfind(|notification| notification.mode == mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn any(title: &str) -> Notification {
        in_mode(title, NotificationMode::Takeover)
    }

    fn in_mode(title: &str, mode: NotificationMode) -> Notification {
        Notification {
            notification_id: 0,
            key: None,
            title: title.to_string(),
            body: None,
            level: Level::Info,
            mode,
            expires_in_seconds: 0,
            starts_at: None,
            meeting: None,
            ends_at: None,
            location: None,
            tab_id: None,
            stinger: None,
        }
    }

    fn keyed(title: &str, key: &str) -> Notification {
        Notification {
            key: Some(key.to_string()),
            ..in_mode(title, NotificationMode::Sidebar)
        }
    }

    #[tokio::test]
    async fn ids_are_handed_out_in_order() {
        let notifications = Notifications::new();

        assert_eq!(
            notifications
                .push(any("one"), Duration::from_secs(60))
                .await,
            1
        );
        assert_eq!(
            notifications
                .push(any("two"), Duration::from_secs(60))
                .await,
            2
        );
    }

    #[tokio::test]
    async fn the_current_one_is_the_newest() {
        let notifications = Notifications::new();

        notifications
            .push(any("one"), Duration::from_secs(60))
            .await;
        notifications
            .push(any("two"), Duration::from_secs(60))
            .await;

        assert_eq!(notifications.current().await.unwrap().title, "two");
    }

    #[tokio::test]
    async fn an_expired_notification_stops_being_current() {
        let notifications = Notifications::new();

        notifications
            .push(any("gone"), Duration::from_millis(1))
            .await;
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert!(notifications.current().await.is_none());
    }

    #[tokio::test]
    async fn dismissing_reports_whether_it_was_there() {
        let notifications = Notifications::new();
        let id = notifications
            .push(any("one"), Duration::from_secs(60))
            .await;

        assert!(notifications.dismiss(id).await);
        assert!(!notifications.dismiss(id).await);
    }

    #[tokio::test]
    async fn remaining_time_counts_down() {
        let notifications = Notifications::new();

        notifications
            .push(any("one"), Duration::from_secs(30))
            .await;

        let remaining = notifications.current().await.unwrap().expires_in_seconds;

        assert!((28..=30).contains(&remaining));
    }

    #[tokio::test]
    async fn a_key_replaces_rather_than_appends() {
        let notifications = Notifications::new();

        let first = notifications
            .push(
                keyed("Standup", "calendar:work:abc"),
                Duration::from_secs(60),
            )
            .await;
        let second = notifications
            .push(
                keyed("Standup, moved", "calendar:work:abc"),
                Duration::from_secs(60),
            )
            .await;

        assert_eq!(first, second);

        let active = notifications.active().await;

        assert_eq!(active.len(), 1);
        assert_eq!(active[0].title, "Standup, moved");
    }

    #[tokio::test]
    async fn a_push_without_a_key_still_appends() {
        let notifications = Notifications::new();

        notifications
            .push(any("one"), Duration::from_secs(60))
            .await;
        notifications
            .push(any("two"), Duration::from_secs(60))
            .await;

        assert_eq!(notifications.active().await.len(), 2);
    }

    #[tokio::test]
    async fn one_mode_expiring_does_not_wait_on_another() {
        let notifications = Notifications::new();

        notifications
            .push(
                in_mode("standup", NotificationMode::Sidebar),
                Duration::from_secs(60),
            )
            .await;
        notifications
            .push(
                in_mode("doorbell", NotificationMode::Takeover),
                Duration::from_millis(1),
            )
            .await;

        tokio::time::sleep(Duration::from_millis(20)).await;

        assert!(notifications
            .current_in(NotificationMode::Takeover)
            .await
            .is_none());
        assert!(notifications
            .current_in(NotificationMode::Sidebar)
            .await
            .is_some());
        assert!(notifications.current().await.is_some());
    }

    #[tokio::test]
    async fn keyed_entries_can_be_retired_as_a_group() {
        let notifications = Notifications::new();

        notifications
            .push(
                keyed("standup", "calendar:work:one"),
                Duration::from_secs(60),
            )
            .await;
        notifications
            .push(
                keyed("review", "calendar:work:two"),
                Duration::from_secs(60),
            )
            .await;
        notifications
            .push(any("doorbell"), Duration::from_secs(60))
            .await;

        notifications
            .retain_keyed(|key| key == "calendar:work:one")
            .await;

        let titles: Vec<_> = notifications
            .active()
            .await
            .into_iter()
            .map(|notification| notification.title)
            .collect();

        assert_eq!(titles, ["standup", "doorbell"]);
    }
}
