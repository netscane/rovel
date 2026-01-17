//! 文本分割器
//!
//! 提供智能文本分段功能，支持最小字符数限制

/// 默认最小字符数限制
/// 当片段字符数未达到此限制时，弱分隔符不会触发分割
pub const DEFAULT_MIN_CHARS: usize = 20;

/// 文本分割配置
#[derive(Debug, Clone)]
pub struct SegmentConfig {
    /// 最小字符数限制（用于合并短句）
    pub min_chars: usize,
}

impl Default for SegmentConfig {
    fn default() -> Self {
        Self {
            min_chars: DEFAULT_MIN_CHARS,
        }
    }
}

/// 检查是否为强分隔符（句末标点，总是分割）
#[inline]
fn is_strong_delimiter(ch: char) -> bool {
    matches!(ch, '。' | '？' | '！' | '.' | '?' | '!')
}

/// 检查是否为弱分隔符（逗号等，达到最小字符数时才分割）
#[inline]
fn is_weak_delimiter(ch: char) -> bool {
    matches!(ch, '，' | '；' | '：' | ',' | ';' | ':')
}

/// 检查片段是否只包含引号或空白（应该被过滤或合并）
#[inline]
fn is_trivial_segment(s: &str) -> bool {
    // 中文引号: " (\u{201C}) " (\u{201D})  中文单引号: ' (\u{2018}) ' (\u{2019})
    s.chars().all(|c| matches!(c, '"' | '\u{201C}' | '\u{201D}' | '\'' | '\u{2018}' | '\u{2019}' | ' ' | '\t'))
}



/// 按标点符号分割单行文本（带最小字符数限制，行内合并短句）
///
/// 分割策略：
/// 1. 按弱分隔符（需满足 min_chars）或强分隔符分割
/// 2. 合并短片段直到满足 min_chars
fn split_line(text: &str, config: &SegmentConfig) -> Vec<String> {
    // 第一步：按标点分割
    let raw_segments = split_by_delimiters(text, config);
    
    // 第二步：合并短片段
    merge_until_min_chars(raw_segments, config.min_chars)
}

/// 按分隔符分割（不做合并）
fn split_by_delimiters(text: &str, config: &SegmentConfig) -> Vec<String> {
    let mut segments: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut char_count = 0;

    for ch in text.chars() {
        current.push(ch);
        char_count += 1;

        let should_split = if is_strong_delimiter(ch) {
            true // 强分隔符总是分割
        } else if is_weak_delimiter(ch) && char_count >= config.min_chars {
            true // 弱分隔符在满足 min_chars 时分割
        } else {
            false
        };

        if should_split {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                segments.push(trimmed);
            }
            current.clear();
            char_count = 0;
        }
    }

    // 剩余内容
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        segments.push(trimmed);
    }

    segments
}

/// 合并短片段直到满足 min_chars
fn merge_until_min_chars(segments: Vec<String>, min_chars: usize) -> Vec<String> {
    if segments.is_empty() {
        return segments;
    }

    let mut result: Vec<String> = Vec::new();
    let mut buffer = String::new();

    for seg in segments {
        buffer.push_str(&seg);
        
        if buffer.chars().count() >= min_chars {
            result.push(std::mem::take(&mut buffer));
        }
    }

    // 处理剩余buffer
    if !buffer.is_empty() {
        if let Some(last) = result.last_mut() {
            // 合并到前一个
            last.push_str(&buffer);
        } else {
            // 没有前一个，单独保留
            result.push(buffer);
        }
    }

    result
}

/// 对文本进行分段
///
/// 分段策略：
/// 1. 按行分割（支持 \n 和 \r\n）
/// 2. 每行按标点符号分割（带最小字符数限制，行内合并短句）
/// 3. 过滤/合并只有引号的片段
pub fn segment_text(text: &str, config: &SegmentConfig) -> Vec<String> {
    let mut segments: Vec<String> = Vec::new();

    // 按行分割
    let lines: Vec<&str> = text
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for line in lines {
        let sentences = split_line(line, config);
        for sentence in sentences {
            let trimmed = sentence.trim();
            if trimmed.is_empty() {
                continue;
            }
            
            // 如果是只有引号的片段，合并到前一个片段
            if is_trivial_segment(trimmed) {
                if let Some(last) = segments.last_mut() {
                    last.push_str(trimmed);
                }
            } else {
                segments.push(trimmed.to_string());
            }
        }
    }

    segments
}

/// 使用默认配置分段（便捷方法）
pub fn segment_text_default(text: &str) -> Vec<String> {
    segment_text(text, &SegmentConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strong_delimiter_always_splits() {
        let config = SegmentConfig { min_chars: 100 }; // 设置很大的限制
        let text = "短。短？短！";
        let segments = split_line(text, &config);

        // 短句会被合并
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0], "短。短？短！");
    }

    #[test]
    fn test_weak_delimiter_respects_min_chars() {
        let config = SegmentConfig { min_chars: 20 };
        // 测试逗号不会在字符数不足时分割
        let text = "所以，如今想要讨还回去吧，苦涩的一笑。";
        let segments = split_line(text, &config);

        // 只有一个片段
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0], "所以，如今想要讨还回去吧，苦涩的一笑。");
    }

    #[test]
    fn test_weak_delimiter_splits_when_enough_chars() {
        let config = SegmentConfig { min_chars: 10 };
        let text = "这是一段很长的文字内容，另一段也很长的内容。";
        let segments = split_line(text, &config);

        // 第一个逗号处超过10字符，应该分割
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], "这是一段很长的文字内容，");
        assert_eq!(segments[1], "另一段也很长的内容。");
    }

    #[test]
    fn test_segment_text_with_lines_no_cross_merge() {
        // 测试跨行不合并
        let config = SegmentConfig { min_chars: 50 };
        let text = "第一行。\n第二行。";
        let segments = segment_text(text, &config);

        // 即使都很短，跨行也不合并
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], "第一行。");
        assert_eq!(segments[1], "第二行。");
    }

    #[test]
    fn test_user_example() {
        let config = SegmentConfig { min_chars: 20 };
        let text = "所以，如今想要讨还回去吧……苦涩的一笑，萧炎落寞的转身，安静地回到了队伍的最后一排，孤单的身影。";
        let segments = split_line(text, &config);

        for seg in &segments {
            println!("segment: {} ({}字)", seg, seg.chars().count());
        }
    }

    #[test]
    fn test_default_config() {
        let segments = segment_text_default("测试内容。");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0], "测试内容。");
    }

    #[test]
    fn test_quote_only_segment_merged() {
        // 测试只有引号的片段会被合并到前一个片段
        let config = SegmentConfig { min_chars: 10 };
        let text = "这是一段较长的内容测试。\n\"\n这是另一段较长的测试内容。";
        let segments = segment_text(text, &config);

        // 单独的 " 应该被合并到前一个片段
        assert_eq!(segments.len(), 2);
        assert!(segments[0].ends_with("\""));
    }

    #[test]
    fn test_trivial_segment_detection() {
        assert!(is_trivial_segment("\""));
        assert!(is_trivial_segment("\" "));
        assert!(!is_trivial_segment("内容"));
    }

    #[test]
    fn test_short_segments_merged_within_line() {
        let config = SegmentConfig { min_chars: 20 };
        // 同一行内的短句应该被合并
        let text = "三段？嘿嘿，果然不出我所料！";
        let segments = segment_text(text, &config);

        // 行内短句合并
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn test_novel_sample() {
        let config = SegmentConfig { min_chars: 20 };
        let text = r#"第001章 陨落的天才

"斗之力，三段！"

望着测验魔石碑上面闪亮得甚至有些刺眼的五个大字，少年面无表情，唇角有着一抹自嘲，紧握的手掌，因为大力，而导致略微尖锐的指甲深深的刺进了掌心之中，带来一阵阵钻心的疼痛。

"三段？嘿嘿，果然不出我所料，这个"天才"这一年又是在原地踏步！""#;
        
        let segments = segment_text(text, &config);
        
        println!("=== Novel Sample Segments ===");
        for (i, seg) in segments.iter().enumerate() {
            println!("[{}] ({} chars): {}", i, seg.chars().count(), seg);
        }
        
        // 每行独立，不跨行合并
        // 第一行: 第001章 陨落的天才
        // 第二行: "斗之力，三段！"
        // 等等...
        assert!(segments.len() >= 4);
    }
}
