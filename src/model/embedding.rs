//! Thin wrapper around the `model2vec-rs` inference crate.

use model2vec_rs::model::StaticModel;

/// Wrapper around a loaded `StaticModel`.
#[derive(Clone)]
pub struct EmbeddingModel {
    inner: StaticModel,
    dimension: usize,
}

impl EmbeddingModel {
    /// Load a model from a Hugging Face id or a local path.
    ///
    /// # Errors
    ///
    /// Returns an error if the model files cannot be found or parsed.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let inner = StaticModel::from_pretrained(path, None, None, None)?;
        let dimension = inner.encode_single("hello").len();

        Ok(Self { inner, dimension })
    }

    /// The model's configured `max_length` from `config.json`, or `None` when
    /// the model declares no limit.
    #[must_use]
    pub fn max_length(&self) -> Option<usize> {
        self.inner.max_length()
    }

    /// The model's configured L2-normalization setting.
    #[must_use]
    pub fn normalize(&self) -> bool {
        self.inner.normalize()
    }

    /// Return the embedding dimension of the loaded model.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Encode a batch of input strings into embeddings.
    ///
    /// `normalize` overrides the model's configured normalization for this
    /// call; `None` keeps the model's configured behavior.
    ///
    /// # Panics
    ///
    /// Never panics; returns an empty vector only if the input is empty.
    #[must_use]
    pub fn encode(
        &self,
        inputs: &[String],
        max_length: usize,
        batch_size: usize,
        normalize: Option<bool>,
    ) -> Vec<Vec<f32>> {
        if inputs.is_empty() {
            return Vec::new();
        }

        self.inner
            .encode_with_args(inputs, Some(max_length), batch_size, normalize)
    }

    /// Encode a single input string.
    ///
    /// `normalize` overrides the model's configured normalization for this
    /// call; `None` keeps the model's configured behavior.
    #[must_use]
    pub fn encode_single(
        &self,
        input: &str,
        max_length: Option<usize>,
        normalize: Option<bool>,
    ) -> Vec<f32> {
        self.inner
            .encode_with_args(&[input.to_owned()], max_length, 1, normalize)
            .into_iter()
            .next()
            .unwrap_or_default()
    }
}
