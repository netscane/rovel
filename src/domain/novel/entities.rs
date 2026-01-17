//! Novel Context - Entities

use serde::{Deserialize, Serialize};

/// 文本片段 - 最小 TTS/播放单位
///
/// 不变量:
/// - index 在 Novel 内唯一且有序
/// - content 不可为空
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSegment {
    /// 片段索引（在小说中的顺序）
    index: usize,
    /// 片段内容
    content: String,
}

impl TextSegment {
    pub fn new(index: usize, content: String) -> Result<Self, &'static str> {
        if content.is_empty() {
            return Err("片段内容不能为空");
        }
        Ok(Self { index, content })
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

/// 章节信息（可选）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chapter {
    /// 章节编号
    number: usize,
    /// 章节标题
    title: String,
    /// 起始片段索引
    start_segment_index: usize,
    /// 结束片段索引（不包含）
    end_segment_index: usize,
}

impl Chapter {
    pub fn new(
        number: usize,
        title: String,
        start_segment_index: usize,
        end_segment_index: usize,
    ) -> Result<Self, &'static str> {
        if start_segment_index >= end_segment_index {
            return Err("章节起始索引必须小于结束索引");
        }
        Ok(Self {
            number,
            title,
            start_segment_index,
            end_segment_index,
        })
    }

    pub fn number(&self) -> usize {
        self.number
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn start_segment_index(&self) -> usize {
        self.start_segment_index
    }

    pub fn end_segment_index(&self) -> usize {
        self.end_segment_index
    }

    pub fn contains_segment(&self, index: usize) -> bool {
        index >= self.start_segment_index && index < self.end_segment_index
    }
}
