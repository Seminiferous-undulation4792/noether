use super::embedding::{Embedding, EmbeddingError, EmbeddingProvider};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

/// Wraps an EmbeddingProvider with a file-backed cache.
/// Embeddings are keyed by SHA-256 of the input text.
pub struct CachedEmbeddingProvider {
    inner: Box<dyn EmbeddingProvider>,
    cache: HashMap<String, Embedding>,
    path: PathBuf,
    dirty: bool,
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    entries: Vec<CacheEntry>,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    text_hash: String,
    embedding: Embedding,
}

impl CachedEmbeddingProvider {
    pub fn new(inner: Box<dyn EmbeddingProvider>, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let cache = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|content| {
                    if content.trim().is_empty() {
                        return None;
                    }
                    serde_json::from_str::<CacheFile>(&content).ok()
                })
                .map(|f| {
                    f.entries
                        .into_iter()
                        .map(|e| (e.text_hash, e.embedding))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self {
            inner,
            cache,
            path,
            dirty: false,
        }
    }

    fn text_hash(text: &str) -> String {
        hex::encode(Sha256::digest(text.as_bytes()))
    }

    /// Flush cache to disk if dirty.
    pub fn flush(&self) {
        if !self.dirty {
            return;
        }
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = CacheFile {
            entries: self
                .cache
                .iter()
                .map(|(h, e)| CacheEntry {
                    text_hash: h.clone(),
                    embedding: e.clone(),
                })
                .collect(),
        };
        if let Ok(json) = serde_json::to_string(&file) {
            let _ = std::fs::write(&self.path, json);
        }
    }
}

impl Drop for CachedEmbeddingProvider {
    fn drop(&mut self) {
        self.flush();
    }
}

impl EmbeddingProvider for CachedEmbeddingProvider {
    fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }

    fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let hash = Self::text_hash(text);
        if let Some(cached) = self.cache.get(&hash) {
            return Ok(cached.clone());
        }
        // Cache miss — compute and store
        // We need interior mutability here since the trait requires &self
        // Use unsafe or switch to RefCell. For simplicity, call inner and
        // let the caller handle caching via embed_and_cache.
        self.inner.embed(text)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
}

impl CachedEmbeddingProvider {
    /// Embed with caching — stores result in cache.
    pub fn embed_cached(&mut self, text: &str) -> Result<Embedding, EmbeddingError> {
        let hash = Self::text_hash(text);
        if let Some(cached) = self.cache.get(&hash) {
            return Ok(cached.clone());
        }
        let embedding = self.inner.embed(text)?;
        self.cache.insert(hash, embedding.clone());
        self.dirty = true;
        Ok(embedding)
    }

    /// Embed many texts at once, calling `inner.embed_batch` on cache
    /// misses. Cache hits are served from memory. Misses are sent in chunks
    /// of `chunk_size` to keep individual requests under typical provider
    /// payload limits and to avoid tripping rate limits with one giant call.
    ///
    /// Order of results matches order of `texts`.
    pub fn embed_batch_cached(
        &mut self,
        texts: &[&str],
        chunk_size: usize,
    ) -> Result<Vec<Embedding>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Identify misses without touching `inner` yet.
        let hashes: Vec<String> = texts.iter().map(|t| Self::text_hash(t)).collect();
        let mut miss_indices: Vec<usize> = Vec::new();
        let mut miss_texts: Vec<&str> = Vec::new();
        for (i, h) in hashes.iter().enumerate() {
            if !self.cache.contains_key(h) {
                miss_indices.push(i);
                miss_texts.push(texts[i]);
            }
        }

        // Resolve misses in chunks.
        if !miss_texts.is_empty() {
            let chunk = chunk_size.max(1);
            let mut filled: Vec<Embedding> = Vec::with_capacity(miss_texts.len());
            for slice in miss_texts.chunks(chunk) {
                let part = self.inner.embed_batch(slice)?;
                filled.extend(part);
            }
            for (idx, emb) in miss_indices.iter().zip(filled.into_iter()) {
                self.cache.insert(hashes[*idx].clone(), emb);
            }
            self.dirty = true;
        }

        // Assemble results in input order.
        Ok(hashes
            .iter()
            .map(|h| self.cache.get(h).cloned().expect("just inserted"))
            .collect())
    }
}
