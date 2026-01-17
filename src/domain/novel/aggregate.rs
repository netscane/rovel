//! Novel Context - Aggregate Root

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Chapter, NovelId, RawTextPath, TextSegment, Title};
use crate::domain::text_segmenter::{segment_text, SegmentConfig};

/// Novel 聚合根
///
/// 不变量:
/// - 小说文本只属于一个 Novel
/// - Segment 顺序不可变
/// - Novel 创建后文本不可被随意修改（除非新版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Novel {
    id: NovelId,
    title: Title,
    raw_text_path: RawTextPath,
    segments: Vec<TextSegment>,
    chapters: Vec<Chapter>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Novel {
    /// 创建新小说
    pub fn new(title: Title, raw_text_path: RawTextPath) -> Self {
        let now = Utc::now();
        Self {
            id: NovelId::new(),
            title,
            raw_text_path,
            segments: Vec::new(),
            chapters: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 从原始文本创建小说并自动分段
    pub fn from_text(title: Title, raw_text_path: RawTextPath, text: &str) -> Self {
        let mut novel = Self::new(title, raw_text_path);
        novel.segment_text(text);
        novel
    }

    /// 对文本进行分段
    ///
    /// 分段策略:
    /// 1. 按行分割（单换行）
    /// 2. 每行按标点符号分割（带最小字符数限制）
    /// 3. 确保每个片段适合 TTS 处理
    pub fn segment_text(&mut self, text: &str) {
        self.segments.clear();

        // 使用共享的分割模块
        let sentences = segment_text(text, &SegmentConfig::default());
        
        for (index, sentence) in sentences.into_iter().enumerate() {
            if let Ok(segment) = TextSegment::new(index, sentence) {
                self.segments.push(segment);
            }
        }

        self.updated_at = Utc::now();
    }

    /// 设置章节信息
    pub fn set_chapters(&mut self, chapters: Vec<Chapter>) {
        self.chapters = chapters;
        self.updated_at = Utc::now();
    }

    // Getters
    pub fn id(&self) -> &NovelId {
        &self.id
    }

    pub fn title(&self) -> &Title {
        &self.title
    }

    pub fn raw_text_path(&self) -> &RawTextPath {
        &self.raw_text_path
    }

    pub fn segments(&self) -> &[TextSegment] {
        &self.segments
    }

    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    pub fn get_segment(&self, index: usize) -> Option<&TextSegment> {
        self.segments.get(index)
    }

    pub fn chapters(&self) -> &[Chapter] {
        &self.chapters
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// 获取指定范围的片段
    pub fn get_segments_range(&self, start: usize, end: usize) -> &[TextSegment] {
        let end = end.min(self.segments.len());
        let start = start.min(end);
        &self.segments[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_novel_creation() {
        let title = Title::new("测试小说").unwrap();
        let path = RawTextPath::from("/tmp/test.txt");
        let novel = Novel::new(title, path);

        assert_eq!(novel.title().as_str(), "测试小说");
        assert!(novel.segments().is_empty());
    }

    #[test]
    fn test_text_segmentation() {
        let title = Title::new("测试小说").unwrap();
        let path = RawTextPath::from("/tmp/test.txt");
        // 使用足够长的句子（>20字符），确保不会被合并
        let text = "这是第一句话内容较长需要超过二十个字符。\n这是第二句话内容也较长需要超过二十个字符。";

        let novel = Novel::from_text(title, path, text);

        // 按句号分割为2段
        assert_eq!(novel.segment_count(), 2);
    }
}
