//! Errors from the Bunny API client (design §3) and their tool-facing text (design §4).

#[derive(Debug, thiserror::Error)]
pub enum BunnyError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound { path: String },
    #[error("rate limited")]
    RateLimited,
    #[error("api error {status}")]
    Api {
        status: u16,
        key: Option<String>,
        field: Option<String>,
        message: Option<String>,
    },
    #[error("timeout")]
    Timeout,
    #[error("network: {0}")]
    Network(String),
    #[error("decode: {0}")]
    Decode(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

impl BunnyError {
    /// Actionable one-line text for `CallToolResult::error` (C4).
    pub fn to_tool_message(&self) -> String {
        match self {
            Self::Unauthorized => "Bunny API rejected the API key (401). Check BUNNY_API_KEY / \
                 --api-key-file: it must be the account API key (Dashboard → Account settings → API)."
                .to_string(),
            Self::NotFound { path } => {
                format!("Not found (404): {path}. Verify the id with bunny_pullzone_list.")
            }
            Self::RateLimited => {
                "Bunny API rate limit hit (429) after 3 retries. Wait a few seconds and retry."
                    .to_string()
            }
            Self::Api {
                status,
                key,
                field,
                message,
            } => {
                let mut s = format!("Bunny API error {status}");
                if let Some(key) = key {
                    s.push_str(&format!(" ({key})"));
                }
                if let Some(field) = field {
                    s.push_str(&format!(" field={field}"));
                }
                s.push_str(": ");
                s.push_str(message.as_deref().unwrap_or("no error message in response"));
                s
            }
            Self::Timeout => "Bunny API did not respond within 30s. The operation may or may not \
                 have been applied; check state before retrying."
                .to_string(),
            Self::Network(detail) => format!("Network error talking to Bunny API: {detail}"),
            Self::Decode(detail) => format!("Unexpected response from Bunny API: {detail}"),
            Self::InvalidArgument(detail) => format!("Invalid argument: {detail}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_message_format() {
        let e = BunnyError::Api {
            status: 400,
            key: Some("pullzone.validation".into()),
            field: Some("OriginUrl".into()),
            message: Some("Invalid origin".into()),
        };
        assert_eq!(
            e.to_tool_message(),
            "Bunny API error 400 (pullzone.validation) field=OriginUrl: Invalid origin"
        );
        let e = BunnyError::Api {
            status: 500,
            key: None,
            field: None,
            message: Some("boom".into()),
        };
        assert_eq!(e.to_tool_message(), "Bunny API error 500: boom");
    }

    #[test]
    fn fixed_messages() {
        assert!(
            BunnyError::Unauthorized
                .to_tool_message()
                .starts_with("Bunny API rejected the API key (401).")
        );
        assert_eq!(
            BunnyError::NotFound {
                path: "/pullzone/7".into()
            }
            .to_tool_message(),
            "Not found (404): /pullzone/7. Verify the id with bunny_pullzone_list."
        );
        assert_eq!(
            BunnyError::InvalidArgument("id must be >= 1".into()).to_tool_message(),
            "Invalid argument: id must be >= 1"
        );
    }
}
