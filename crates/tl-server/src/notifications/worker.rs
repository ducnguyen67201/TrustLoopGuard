use std::{sync::Arc, time::Duration};

use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use super::{ClaimedNotification, NotificationStore};

#[derive(Debug, Clone)]
pub struct NotificationWorkerConfig {
    pub poll_interval: Duration,
    pub lease_seconds: i64,
    pub max_attempts: i32,
    pub dashboard_url: String,
    pub from: String,
}

impl NotificationWorkerConfig {
    pub fn from_env() -> Option<(Self, AsyncSmtpTransport<Tokio1Executor>)> {
        let smtp_url = std::env::var("TL_NOTIFICATION_SMTP_URL").ok()?;
        let from = std::env::var("TL_NOTIFICATION_EMAIL_FROM").ok()?;
        let dashboard_url =
            std::env::var("TL_DASHBOARD_URL").unwrap_or_else(|_| "http://localhost:3000".into());
        let parsed = validated_smtp_endpoint(&smtp_url, &from)?;
        let host = parsed.host_str()?;
        let mut builder = if parsed.scheme() == "smtps" {
            AsyncSmtpTransport::<Tokio1Executor>::relay(host).ok()?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
        };
        if let Some(port) = parsed.port() {
            builder = builder.port(port);
        }
        if !parsed.username().is_empty() {
            builder = builder.credentials(Credentials::new(
                parsed.username().to_string(),
                parsed.password().unwrap_or_default().to_string(),
            ));
        }
        Some((
            Self {
                poll_interval: Duration::from_millis(500),
                lease_seconds: 90,
                max_attempts: 5,
                dashboard_url,
                from,
            },
            builder.build(),
        ))
    }
}

fn validated_smtp_endpoint(smtp_url: &str, from: &str) -> Option<url::Url> {
    let parsed = url::Url::parse(smtp_url).ok()?;
    if !matches!(parsed.scheme(), "smtp" | "smtps") {
        return None;
    }
    let _: Mailbox = from.parse().ok()?;
    Some(parsed)
}

pub fn spawn_notification_worker(
    store: Arc<dyn NotificationStore>,
    config: NotificationWorkerConfig,
    transport: AsyncSmtpTransport<Tokio1Executor>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let worker_id = format!("notification-worker-{}", uuid::Uuid::now_v7());
        let mut interval = tokio::time::interval(config.poll_interval);
        loop {
            interval.tick().await;
            match store.claim(&worker_id, config.lease_seconds).await {
                Ok(Some(claimed)) => deliver(store.as_ref(), &config, &transport, claimed).await,
                Ok(None) => {}
                Err(error) => tracing::warn!(error = %error, "notification claim failed"),
            }
        }
    })
}

async fn deliver(
    store: &dyn NotificationStore,
    config: &NotificationWorkerConfig,
    transport: &AsyncSmtpTransport<Tokio1Executor>,
    claimed: ClaimedNotification,
) {
    let result = build_message(config, &claimed).map_err(|_| {
        (
            "message_build",
            "notification message could not be built".to_string(),
        )
    });
    let result = match result {
        Ok(message) => transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|_| ("smtp_delivery", "SMTP delivery failed".to_string())),
        Err(error) => Err(error),
    };
    match result {
        Ok(()) => {
            if let Err(error) = store
                .mark_sent(&claimed.workspace_id, &claimed.delivery.id)
                .await
            {
                tracing::warn!(delivery_id = %claimed.delivery.id, error = %error, "notification sent but could not be marked sent");
            }
        }
        Err((code, message)) => {
            if let Err(error) = store
                .retry_or_fail(
                    &claimed.workspace_id,
                    &claimed.delivery.id,
                    config.max_attempts,
                    code,
                    &message,
                )
                .await
            {
                tracing::warn!(delivery_id = %claimed.delivery.id, error = %error, "notification retry transition failed");
            }
        }
    }
}

fn build_message(
    config: &NotificationWorkerConfig,
    claimed: &ClaimedNotification,
) -> Result<Message, ()> {
    let from: Mailbox = config.from.parse().map_err(|_| ())?;
    let to: Mailbox = claimed.email.parse().map_err(|_| ())?;
    let title = claimed
        .payload
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Featherlane production alert");
    let detail = claimed
        .payload
        .get("detail")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("A production Run needs attention.");
    let run_link = claimed.run_id.as_ref().map(|run_id| {
        format!(
            "{}/runs/{run_id}",
            config.dashboard_url.trim_end_matches('/')
        )
    });
    let body = match &run_link {
        Some(link) => format!(
            "{detail}\n\nOpen the Run: {link}\n\nDelivery ID: {}",
            claimed.delivery.id
        ),
        None => format!("{detail}\n\nDelivery ID: {}", claimed.delivery.id),
    };
    let html_link = run_link
        .map(|link| format!("<p><a href=\"{}\">Open the Run</a></p>", escape_html(&link)))
        .unwrap_or_default();
    let html = format!(
        "<p>{}</p>{html_link}<p>Delivery ID: {}</p>",
        escape_html(detail),
        escape_html(&claimed.delivery.id)
    );
    Message::builder()
        .from(from)
        .to(to)
        .subject(title)
        .message_id(Some(format!(
            "<{}@notifications.featherlane.ai>",
            claimed.delivery.id
        )))
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(body),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html),
                ),
        )
        .map_err(|_| ())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::{escape_html, validated_smtp_endpoint};

    #[test]
    fn notification_html_escapes_untrusted_values() {
        assert_eq!(
            escape_html("<script>alert('x') & \"y\"</script>"),
            "&lt;script&gt;alert(&#39;x&#39;) &amp; &quot;y&quot;&lt;/script&gt;"
        );
    }

    #[test]
    fn smtp_readiness_requires_supported_url_and_valid_sender() {
        assert!(
            validated_smtp_endpoint("smtp://mail.example.com:2525", "alerts@example.com").is_some()
        );
        assert!(validated_smtp_endpoint("http://mail.example.com", "alerts@example.com").is_none());
        assert!(validated_smtp_endpoint("smtp://mail.example.com", "not an email").is_none());
    }
}
