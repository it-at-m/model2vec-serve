//! Runtime configuration parsed from CLI arguments and environment variables.

use clap::Parser;
use std::path::PathBuf;

/// Fallback maximum input length used when neither the operator nor the
/// model's `config.json` declares a limit.
pub const DEFAULT_MAX_INPUT_LENGTH: usize = 512;

/// Configuration for the model2vec-serve HTTP server.
#[derive(Parser, Clone, Debug)]
#[command(name = "model2vec-serve", version, about)]
pub struct Config {
    /// Network interface to bind to.
    #[arg(long, default_value = "0.0.0.0", env = "HOST")]
    pub host: String,

    /// Port to listen on.
    #[arg(long, default_value_t = 8080, env = "PORT")]
    pub port: u16,

    /// Hugging Face model id or local path to a model2vec model directory.
    ///
    /// May be provided multiple times or as a comma-separated list via the
    /// `MODEL` environment variable.
    #[arg(
        long = "model",
        default_value = "minishlab/blink-multilingual-64M",
        env = "MODEL",
        value_delimiter = ','
    )]
    pub models: Vec<String>,

    /// Model identifier to use when a request does not specify one.
    /// Defaults to the first entry in `--model` if omitted.
    #[arg(long = "default-model", env = "DEFAULT_MODEL")]
    pub default_model: Option<String>,

    /// Model publisher or owner shown in OpenAI-compatible `/v1/models` responses.
    #[arg(long = "model-owner", env = "MODEL_OWNER", default_value = "minishlab")]
    pub model_owner: String,

    /// Path identifier alias for a model, as `KEY=ALIAS`.
    ///
    /// `KEY` identifies the target model: either the model identifier or
    /// local path exactly as configured via `--model`, or the derived
    /// canonical identifier (for local paths, the directory name). `ALIAS`
    /// overrides the path segment used by the `/tei/{model_id}/...`
    /// endpoints. May be provided multiple times or as a comma-separated
    /// list via the `MODEL_ALIAS` environment variable.
    #[arg(
        long = "model-alias",
        env = "MODEL_ALIAS",
        value_delimiter = ',',
        value_parser = parse_model_alias
    )]
    pub model_alias: Vec<(String, String)>,

    /// Optional API key. When set, embedding endpoints require a Bearer token.
    #[arg(long, env = "API_KEY")]
    pub api_key: Option<String>,

    /// Maximum number of input strings accepted in a single request.
    #[arg(long, default_value_t = 256, env = "MAX_BATCH_SIZE")]
    pub max_batch_size: usize,

    /// Maximum tokens per input string.
    ///
    /// When set (explicitly via `--max-input-length` or `MAX_INPUT_LENGTH`),
    /// the value overrides every model's configured limit. When unset, each
    /// model's limit resolves from its own `config.json` `max_length`,
    /// falling back to [`DEFAULT_MAX_INPUT_LENGTH`].
    #[arg(long, env = "MAX_INPUT_LENGTH")]
    pub max_input_length: Option<usize>,

    /// Log level (e.g., trace, debug, info, warn, error).
    #[arg(long, default_value = "info", env = "LOG_LEVEL")]
    pub log_level: String,

    /// Per-request timeout in seconds.
    #[arg(long, default_value_t = 30, env = "REQUEST_TIMEOUT_SECONDS")]
    pub request_timeout_seconds: u64,

    /// Path to the PEM-encoded TLS certificate (leaf plus optional chain).
    ///
    /// TLS is enabled only when both this and `--tls-key` are provided.
    #[arg(long = "tls-cert", env = "TLS_CERT")]
    pub tls_cert: Option<PathBuf>,

    /// Path to the unencrypted PEM private key matching `--tls-cert`.
    ///
    /// TLS is enabled only when both this and `--tls-cert` are provided.
    #[arg(long = "tls-key", env = "TLS_KEY")]
    pub tls_key: Option<PathBuf>,
}

/// Transport mode derived from the TLS options.
///
/// The listener serves either plain HTTP (default) or HTTPS; there is no
/// dual-protocol mode.
#[derive(Clone, Debug)]
pub enum TlsMode {
    /// TLS is disabled: the configured port serves plain HTTP.
    Disabled,
    /// TLS is enabled: the configured port serves HTTPS using the files.
    Enabled {
        /// Path to the PEM certificate file.
        cert: PathBuf,
        /// Path to the matching PEM private key file.
        key: PathBuf,
    },
}

/// Parse a `KEY=ALIAS` pair for [`Config::model_alias`].
///
/// # Errors
///
/// Returns a message describing the first validation failure: missing `=`,
/// empty key or alias, or an alias that is not a single URL path segment.
fn parse_model_alias(pair: &str) -> Result<(String, String), String> {
    let Some((key, alias)) = pair.split_once('=') else {
        return Err(format!("invalid model alias '{pair}': expected KEY=ALIAS"));
    };

    let key = key.trim();
    let alias = alias.trim();

    if key.is_empty() {
        return Err(format!(
            "invalid model alias '{pair}': key must not be empty"
        ));
    }

    if alias.is_empty() {
        return Err(format!(
            "invalid model alias '{pair}': alias must not be empty"
        ));
    }

    if alias.chars().any(char::is_whitespace) {
        return Err(format!(
            "invalid model alias '{pair}': alias must not contain whitespace"
        ));
    }

    if alias.contains('/') {
        return Err(format!(
            "invalid model alias '{pair}': alias must be a single path segment without '/'"
        ));
    }

    Ok((key.to_string(), alias.to_string()))
}

/// Resolve the effective maximum input length for a single model.
///
/// Precedence (spec 008, `contracts/config.md`):
///
/// 1. An explicit operator limit wins over any model configuration.
/// 2. Otherwise the model's configured `max_length` (`config.json`) is used.
/// 3. Otherwise [`DEFAULT_MAX_INPUT_LENGTH`] applies; a model that declares
///    no limit never runs with unbounded truncation.
#[must_use]
pub fn resolve_max_input_length(explicit: Option<usize>, model_max: Option<usize>) -> usize {
    explicit.or(model_max).unwrap_or(DEFAULT_MAX_INPUT_LENGTH)
}

impl Config {
    /// Validate cross-field configuration rules.
    ///
    /// # Errors
    ///
    /// Returns a message describing the first validation failure.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_input_length == Some(0) {
            return Err(
                "--max-input-length must be greater than zero; a zero token limit cannot produce embeddings"
                    .to_string(),
            );
        }

        Ok(())
    }

    /// Returns the socket address the server should bind to.
    #[must_use]
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Returns the model identifier to use when a request does not specify one.
    ///
    /// Returns `None` only when no models are configured, which is not a valid
    /// runtime state.
    #[must_use]
    pub fn default_model(&self) -> Option<String> {
        self.default_model
            .clone()
            .or_else(|| self.models.first().cloned())
    }

    /// Returns the resolved transport mode for the listener.
    ///
    /// TLS is enabled only when both the certificate and the private key are
    /// configured (all-or-nothing rule); supplying exactly one is a
    /// configuration error reported before the listener binds.
    ///
    /// # Errors
    ///
    /// Returns the contract error message when exactly one TLS file is set.
    pub fn tls_mode(&self) -> Result<TlsMode, String> {
        match (&self.tls_cert, &self.tls_key) {
            (None, None) => Ok(TlsMode::Disabled),
            (Some(cert), Some(key)) => Ok(TlsMode::Enabled {
                cert: cert.clone(),
                key: key.clone(),
            }),
            (Some(_), None) => Err(
                "TLS requires both --tls-cert and --tls-key; only --tls-cert was provided"
                    .to_string(),
            ),
            (None, Some(_)) => Err(
                "TLS requires both --tls-cert and --tls-key; only --tls-key was provided"
                    .to_string(),
            ),
        }
    }
}
