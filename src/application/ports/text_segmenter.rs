//! Text Segmenter Port - 文本分割抽象
//!
//! 定义文本分割的抽象接口，具体实现在 infrastructure/adapters 层

/// 分割后的文本片段
#[derive(Debug, Clone)]
pub struct SegmentedText {
    pub index: usize,
    pub content: String,
}

/// 分割配置
#[derive(Debug, Clone)]
pub struct SegmentConfig {
    /// 强分隔符（总是分割）
    pub strong_delimiters: Vec<char>,
    /// 弱分隔符（达到最小字符数后分割）
    pub weak_delimiters: Vec<char>,
    /// 使用弱分隔符的最小字符数
    pub min_chars_for_weak: usize,
    /// 最大片段字符数
    pub max_segment_chars: usize,
}

impl Default for SegmentConfig {
    fn default() -> Self {
        Self {
            strong_delimiters: vec!['。', '？', '！', '.', '?', '!'],
            weak_delimiters: vec!['，', '；', '：', ',', ';', ':'],
            min_chars_for_weak: 20,
            max_segment_chars: 500,
        }
    }
}

/// Text Segmenter Port
///
/// 文本分割器接口
pub trait TextSegmenterPort: Send + Sync {
    /// 将文本分割成片段
    fn segment(&self, text: &str, config: &SegmentConfig) -> Vec<SegmentedText>;
}
