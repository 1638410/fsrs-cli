use sha2::{Digest, Sha256};

/// 计算问题文本的 SHA256 哈希（前 16 位）
pub fn question_hash(question: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(question.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

/// 验证问题哈希是否匹配
pub fn verify_hash(question: &str, expected_hash: &str) -> bool {
    question_hash(question) == expected_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_question_hash_deterministic() {
        let q = "Rust 中一条值同时只能有几个可变引用？";
        let h1 = question_hash(q);
        let h2 = question_hash(q);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_question_hash_different_for_different_input() {
        let h1 = question_hash("问题A");
        let h2 = question_hash("问题B");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_verify_hash() {
        let q = "什么是所有权？";
        let h = question_hash(q);
        assert!(verify_hash(q, &h));
        assert!(!verify_hash("别的问题", &h));
    }
}
