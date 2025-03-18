use rig::embeddings::Embedding;

/// Helper functions for converting between Embedding and Vec<f32>
pub trait EmbeddingConversion {
    fn to_vec(&self) -> Vec<f32>;
    fn from_vec(vec: Vec<f32>) -> Self;
    fn to_binary(&self) -> Vec<u8>;
    fn from_binary(binary: &[u8]) -> Self;
}

impl EmbeddingConversion for Embedding {
    fn to_vec(&self) -> Vec<f32> {
        self.vec.iter().map(|f| *f as f32).collect()
    }

    fn from_vec(vec: Vec<f32>) -> Self {
        Self {
            vec: vec.into_iter().map(|f| f as f64).collect(),
            document: "".to_string(),
        }
    }

    fn to_binary(&self) -> Vec<u8> {
        self.vec
            .iter()
            .flat_map(|f| (*f as f32).to_le_bytes())
            .collect()
    }

    fn from_binary(binary: &[u8]) -> Self {
        let mut vec = Vec::with_capacity(binary.len() / 4);
        for chunk in binary.chunks_exact(4) {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(chunk);
            vec.push(f32::from_le_bytes(bytes));
        }
        Self::from_vec(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_conversion() {
        let original_vec = vec![1.0, 2.0, 3.0];
        let embedding = Embedding::from_vec(original_vec.clone());

        // Test to_vec
        assert_eq!(embedding.to_vec(), original_vec);

        // Test binary conversion
        let binary = embedding.to_binary();
        let recovered_vec = Embedding::from_binary(&binary);
        assert_eq!(recovered_vec.to_vec(), original_vec);
    }
}
