//! WAV Transcoder - 基于 symphonia 的音频转码器
//!
//! 支持：
//! - WAV 解析和信息提取
//! - WAV pass-through（不转码）
//! - WAV → Opus (OGG 容器) 编码

use async_trait::async_trait;
use ogg::writing::PacketWriter;
use opus::{Application, Channels, Encoder};
use std::io::Cursor;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::application::ports::{
    AudioFormat, AudioInfo, AudioTranscoderPort, TranscodeConfig, TranscodeError, TranscodeResult,
};

/// WAV 转码器
///
/// 基于 symphonia 实现的音频转码器
/// 当前主要用于 WAV 解析，后续可扩展支持更多格式
pub struct WavTranscoder {
    /// 是否启用转码（如果为 false，总是返回原始 WAV）
    enabled: bool,
}

impl WavTranscoder {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// 解析 WAV 文件头
    fn parse_wav_header(&self, data: &[u8]) -> Result<WavHeader, TranscodeError> {
        if data.len() < 44 {
            return Err(TranscodeError::InvalidInput(
                "WAV data too short".to_string(),
            ));
        }

        // 验证 RIFF 头
        if &data[0..4] != b"RIFF" {
            return Err(TranscodeError::InvalidInput(
                "Invalid WAV: missing RIFF header".to_string(),
            ));
        }

        // 验证 WAVE 标识
        if &data[8..12] != b"WAVE" {
            return Err(TranscodeError::InvalidInput(
                "Invalid WAV: missing WAVE identifier".to_string(),
            ));
        }

        // 查找 fmt chunk
        let mut pos = 12;
        let mut fmt_chunk: Option<FmtChunk> = None;
        let mut data_start = 0;
        let mut data_size = 0;

        while pos < data.len() - 8 {
            let chunk_id = &data[pos..pos + 4];
            let chunk_size =
                u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                    as usize;

            match chunk_id {
                b"fmt " => {
                    if chunk_size < 16 {
                        return Err(TranscodeError::InvalidInput(
                            "Invalid fmt chunk size".to_string(),
                        ));
                    }
                    let fmt_data = &data[pos + 8..pos + 8 + chunk_size.min(16)];
                    fmt_chunk = Some(FmtChunk {
                        audio_format: u16::from_le_bytes([fmt_data[0], fmt_data[1]]),
                        num_channels: u16::from_le_bytes([fmt_data[2], fmt_data[3]]),
                        sample_rate: u32::from_le_bytes([
                            fmt_data[4],
                            fmt_data[5],
                            fmt_data[6],
                            fmt_data[7],
                        ]),
                        byte_rate: u32::from_le_bytes([
                            fmt_data[8],
                            fmt_data[9],
                            fmt_data[10],
                            fmt_data[11],
                        ]),
                        block_align: u16::from_le_bytes([fmt_data[12], fmt_data[13]]),
                        bits_per_sample: u16::from_le_bytes([fmt_data[14], fmt_data[15]]),
                    });
                }
                b"data" => {
                    data_start = pos + 8;
                    data_size = chunk_size;
                    break;
                }
                _ => {}
            }

            pos += 8 + chunk_size;
            // 对齐到偶数字节
            if chunk_size % 2 != 0 {
                pos += 1;
            }
        }

        let fmt = fmt_chunk.ok_or_else(|| {
            TranscodeError::InvalidInput("Invalid WAV: missing fmt chunk".to_string())
        })?;

        if data_size == 0 {
            return Err(TranscodeError::InvalidInput(
                "Invalid WAV: missing data chunk".to_string(),
            ));
        }

        Ok(WavHeader {
            fmt,
            data_start,
            data_size,
        })
    }

    /// 使用 symphonia 解码 WAV 获取 PCM 数据
    fn decode_wav_to_pcm(&self, data: &[u8]) -> Result<DecodedAudio, TranscodeError> {
        let cursor = Cursor::new(data.to_vec());
        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

        let mut hint = Hint::new();
        hint.with_extension("wav");

        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| TranscodeError::DecodingError(format!("Probe failed: {}", e)))?;

        let mut format = probed.format;

        let track = format
            .default_track()
            .ok_or_else(|| TranscodeError::DecodingError("No audio track found".to_string()))?;

        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or_else(|| TranscodeError::DecodingError("Unknown sample rate".to_string()))?;

        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u8)
            .ok_or_else(|| TranscodeError::DecodingError("Unknown channel count".to_string()))?;

        let decoder_opts = DecoderOptions::default();
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)
            .map_err(|e| TranscodeError::DecodingError(format!("Decoder creation failed: {}", e)))?;

        let mut samples: Vec<f32> = Vec::new();
        let track_id = track.id;

        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => {
                    return Err(TranscodeError::DecodingError(format!(
                        "Packet read error: {}",
                        e
                    )));
                }
            };

            if packet.track_id() != track_id {
                continue;
            }

            let decoded = match decoder.decode(&packet) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Decode error (skipping packet): {}", e);
                    continue;
                }
            };

            let spec = *decoded.spec();
            let num_frames = decoded.frames();
            let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);
            // Only take the actual samples, not the entire buffer capacity
            let actual_samples = num_frames * spec.channels.count();
            samples.extend(&sample_buf.samples()[..actual_samples]);
        }

        let duration_ms = if sample_rate > 0 && channels > 0 {
            (samples.len() as u64 * 1000) / (sample_rate as u64 * channels as u64)
        } else {
            0
        };

        Ok(DecodedAudio {
            samples,
            sample_rate,
            channels,
            duration_ms,
        })
    }

    /// 将 PCM f32 样本编码为 WAV
    fn encode_wav(&self, pcm: &DecodedAudio) -> Result<Vec<u8>, TranscodeError> {
        let bits_per_sample: u16 = 16;
        let num_channels = pcm.channels as u16;
        let sample_rate = pcm.sample_rate;
        let byte_rate = sample_rate * num_channels as u32 * (bits_per_sample / 8) as u32;
        let block_align = num_channels * (bits_per_sample / 8);

        // 转换 f32 样本到 i16
        let pcm_data: Vec<i16> = pcm
            .samples
            .iter()
            .map(|&s| {
                let clamped = s.clamp(-1.0, 1.0);
                (clamped * 32767.0) as i16
            })
            .collect();

        let data_size = pcm_data.len() * 2;
        let file_size = 36 + data_size;

        let mut wav = Vec::with_capacity(44 + data_size);

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(file_size as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // fmt chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format
        wav.extend_from_slice(&num_channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());

        // data chunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data_size as u32).to_le_bytes());

        // PCM data
        for sample in pcm_data {
            wav.extend_from_slice(&sample.to_le_bytes());
        }

        Ok(wav)
    }

    /// 将 PCM f32 样本编码为 Opus (OGG 容器)
    fn encode_opus(
        &self,
        pcm: &DecodedAudio,
        bitrate: u32,
    ) -> Result<Vec<u8>, TranscodeError> {
        // Opus 支持的采样率: 8000, 12000, 16000, 24000, 48000
        // 为了兼容性，如果不在列表中需要重采样
        let target_sample_rate = self.get_opus_compatible_sample_rate(pcm.sample_rate);
        
        // 重采样（如果需要）
        let (samples, sample_rate) = if target_sample_rate != pcm.sample_rate {
            let resampled = self.resample(&pcm.samples, pcm.sample_rate, target_sample_rate, pcm.channels)?;
            (resampled, target_sample_rate)
        } else {
            (pcm.samples.clone(), pcm.sample_rate)
        };

        // Opus 仅支持单声道或立体声
        let channels = if pcm.channels == 1 {
            Channels::Mono
        } else {
            Channels::Stereo
        };
        let channel_count = if pcm.channels == 1 { 1 } else { 2 };

        // 创建 Opus 编码器 (Application::Voip 优化语音)
        let mut encoder = Encoder::new(sample_rate, channels, Application::Voip)
            .map_err(|e| TranscodeError::EncodingError(format!("Failed to create Opus encoder: {}", e)))?;

        // 设置比特率
        encoder
            .set_bitrate(opus::Bitrate::Bits(bitrate as i32))
            .map_err(|e| TranscodeError::EncodingError(format!("Failed to set bitrate: {}", e)))?;

        // 获取编码器延迟 (lookahead) 作为 pre-skip
        // Opus 编码器通常有 ~312 samples @ 48kHz 的延迟
        let pre_skip = encoder.get_lookahead()
            .map(|l| l as u16)
            .unwrap_or(312); // 默认值

        // 转换 f32 到 i16
        let pcm_i16: Vec<i16> = samples
            .iter()
            .map(|&s| {
                let clamped = s.clamp(-1.0, 1.0);
                (clamped * 32767.0) as i16
            })
            .collect();

        // Opus frame size: 支持 2.5, 5, 10, 20, 40, 60 ms
        // 使用 20ms frame (sample_rate * 0.02)
        let frame_size = (sample_rate as usize * 20) / 1000;
        let samples_per_frame = frame_size * channel_count;

        // 创建 OGG writer
        let mut ogg_data = Vec::new();
        {
            let mut packet_writer = PacketWriter::new(&mut ogg_data);
            
            // 写入 Opus Head 包 (RFC 7845)
            let opus_head = self.create_opus_head(channel_count as u8, sample_rate, pre_skip);
            packet_writer
                .write_packet(opus_head, 0, ogg::PacketWriteEndInfo::EndPage, 0)
                .map_err(|e| TranscodeError::EncodingError(format!("Failed to write Opus head: {}", e)))?;

            // 写入 Opus Tags 包
            let opus_tags = self.create_opus_tags();
            packet_writer
                .write_packet(opus_tags, 0, ogg::PacketWriteEndInfo::EndPage, 0)
                .map_err(|e| TranscodeError::EncodingError(format!("Failed to write Opus tags: {}", e)))?;

            // 编码音频数据
            let mut output_buf = vec![0u8; 4000]; // Opus 最大包大小
            
            // RFC 7845: granule position 必须是 48kHz 采样率下的样本数
            // 需要将实际采样率的帧大小转换为 48kHz
            let granule_scale = 48000.0 / sample_rate as f64;
            let frame_granule = (frame_size as f64 * granule_scale) as u64;
            
            // pre_skip 也是 48kHz 下的样本数
            let pre_skip_48k = (pre_skip as f64 * granule_scale) as u64;
            let mut granule_pos: u64 = pre_skip_48k;
            
            // 收集所有 chunks（包括不完整的最后一帧）
            let chunks: Vec<_> = pcm_i16.chunks(samples_per_frame).collect();
            
            // 计算需要刷新的额外帧数（编码器延迟）
            // pre_skip 样本被缓存在编码器中，需要额外的帧来刷新
            let flush_frames = (pre_skip as usize + samples_per_frame - 1) / samples_per_frame;

            for chunk in chunks.into_iter() {
                // 如果最后一帧不完整，用零填充
                let frame = if chunk.len() < samples_per_frame {
                    let mut padded = chunk.to_vec();
                    padded.resize(samples_per_frame, 0);
                    padded
                } else {
                    chunk.to_vec()
                };

                let encoded_len = encoder
                    .encode(&frame, &mut output_buf)
                    .map_err(|e| TranscodeError::EncodingError(format!("Opus encode failed: {}", e)))?;

                granule_pos += frame_granule;
                
                packet_writer
                    .write_packet(
                        output_buf[..encoded_len].to_vec(),
                        0,
                        ogg::PacketWriteEndInfo::NormalPacket,
                        granule_pos,
                    )
                    .map_err(|e| TranscodeError::EncodingError(format!("Failed to write Opus packet: {}", e)))?;
            }
            
            // 刷新编码器：发送额外的静音帧来获取编码器缓冲区中剩余的样本
            let silence_frame = vec![0i16; samples_per_frame];
            for flush_idx in 0..flush_frames {
                let encoded_len = encoder
                    .encode(&silence_frame, &mut output_buf)
                    .map_err(|e| TranscodeError::EncodingError(format!("Opus flush encode failed: {}", e)))?;

                granule_pos += frame_granule;
                
                let is_last = flush_idx == flush_frames - 1;
                let end_info = if is_last {
                    ogg::PacketWriteEndInfo::EndStream
                } else {
                    ogg::PacketWriteEndInfo::NormalPacket
                };

                packet_writer
                    .write_packet(
                        output_buf[..encoded_len].to_vec(),
                        0,
                        end_info,
                        granule_pos,
                    )
                    .map_err(|e| TranscodeError::EncodingError(format!("Failed to write Opus flush packet: {}", e)))?;
            }
        }

        Ok(ogg_data)
    }

    /// 获取 Opus 兼容的采样率
    fn get_opus_compatible_sample_rate(&self, sample_rate: u32) -> u32 {
        // Opus 支持: 8000, 12000, 16000, 24000, 48000
        match sample_rate {
            8000 | 12000 | 16000 | 24000 | 48000 => sample_rate,
            r if r <= 8000 => 8000,
            r if r <= 12000 => 12000,
            r if r <= 16000 => 16000,
            r if r <= 24000 => 24000,
            _ => 48000,
        }
    }

    /// 简单线性重采样
    fn resample(
        &self,
        samples: &[f32],
        from_rate: u32,
        to_rate: u32,
        channels: u8,
    ) -> Result<Vec<f32>, TranscodeError> {
        if from_rate == to_rate {
            return Ok(samples.to_vec());
        }

        let ratio = to_rate as f64 / from_rate as f64;
        let channel_count = channels as usize;
        let frame_count = samples.len() / channel_count;
        let new_frame_count = (frame_count as f64 * ratio) as usize;
        let mut resampled = Vec::with_capacity(new_frame_count * channel_count);

        for i in 0..new_frame_count {
            let src_pos = i as f64 / ratio;
            let src_idx = src_pos as usize;
            let frac = src_pos - src_idx as f64;

            for ch in 0..channel_count {
                let idx0 = src_idx * channel_count + ch;
                let idx1 = ((src_idx + 1).min(frame_count - 1)) * channel_count + ch;

                let s0 = samples.get(idx0).copied().unwrap_or(0.0);
                let s1 = samples.get(idx1).copied().unwrap_or(s0);

                // 线性插值
                let value = s0 + (s1 - s0) * frac as f32;
                resampled.push(value);
            }
        }

        Ok(resampled)
    }

    /// 创建 Opus Head 包 (RFC 7845)
    fn create_opus_head(&self, channels: u8, sample_rate: u32, pre_skip: u16) -> Vec<u8> {
        let mut head = Vec::with_capacity(19);
        head.extend_from_slice(b"OpusHead");  // Magic signature
        head.push(1);                          // Version
        head.push(channels);                   // Channel count
        head.extend_from_slice(&pre_skip.to_le_bytes()); // Pre-skip (encoder delay)
        head.extend_from_slice(&sample_rate.to_le_bytes()); // Input sample rate
        head.extend_from_slice(&0i16.to_le_bytes()); // Output gain
        head.push(0);                          // Channel mapping family
        head
    }

    /// 创建 Opus Tags 包
    fn create_opus_tags(&self) -> Vec<u8> {
        let vendor = "rovel";
        let mut tags = Vec::new();
        tags.extend_from_slice(b"OpusTags");
        tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        tags.extend_from_slice(vendor.as_bytes());
        tags.extend_from_slice(&0u32.to_le_bytes()); // No user comments
        tags
    }
}

#[derive(Debug)]
struct WavHeader {
    fmt: FmtChunk,
    #[allow(dead_code)]
    data_start: usize,
    data_size: usize,
}

#[derive(Debug)]
struct FmtChunk {
    #[allow(dead_code)]
    audio_format: u16,
    num_channels: u16,
    sample_rate: u32,
    #[allow(dead_code)]
    byte_rate: u32,
    #[allow(dead_code)]
    block_align: u16,
    bits_per_sample: u16,
}

#[derive(Debug)]
struct DecodedAudio {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u8,
    duration_ms: u64,
}

#[async_trait]
impl AudioTranscoderPort for WavTranscoder {
    async fn transcode(
        &self,
        wav_data: &[u8],
        config: &TranscodeConfig,
    ) -> Result<TranscodeResult, TranscodeError> {
        let original_size = wav_data.len();

        // 如果未启用转码或目标格式是 WAV，直接返回
        if !self.enabled || config.format == AudioFormat::Wav {
            let info = self.get_audio_info(wav_data)?;
            return Ok(TranscodeResult {
                audio_data: wav_data.to_vec(),
                format: AudioFormat::Wav,
                duration_ms: info.duration_ms,
                sample_rate: info.sample_rate,
                channels: info.channels,
                original_size,
                transcoded_size: original_size,
            });
        }

        // 解码 WAV
        let decoded = self.decode_wav_to_pcm(wav_data)?;

        // 根据目标格式进行编码
        match config.format {
            AudioFormat::Wav => {
                // 如果需要重采样或改变声道，处理后重新编码为 WAV
                let output = self.encode_wav(&decoded)?;
                Ok(TranscodeResult {
                    audio_data: output.clone(),
                    format: AudioFormat::Wav,
                    duration_ms: decoded.duration_ms,
                    sample_rate: decoded.sample_rate,
                    channels: decoded.channels,
                    original_size,
                    transcoded_size: output.len(),
                })
            }
            AudioFormat::Opus => {
                let bitrate = config.bitrate.unwrap_or(32000);
                let opus_data = self.encode_opus(&decoded, bitrate)?;
                
                tracing::debug!(
                    original_size = original_size,
                    opus_size = opus_data.len(),
                    bitrate = bitrate,
                    "Encoded to Opus"
                );

                Ok(TranscodeResult {
                    audio_data: opus_data.clone(),
                    format: AudioFormat::Opus,
                    duration_ms: decoded.duration_ms,
                    sample_rate: decoded.sample_rate,
                    channels: decoded.channels,
                    original_size,
                    transcoded_size: opus_data.len(),
                })
            }
            AudioFormat::Mp3 => {
                // TODO: 实现 MP3 编码
                // 需要添加 mp3lame-encoder 或类似 crate
                tracing::warn!(
                    "MP3 encoding not yet implemented, returning original WAV. \
                     To enable MP3, add an MP3 encoder crate dependency."
                );
                let info = self.get_audio_info(wav_data)?;
                Ok(TranscodeResult {
                    audio_data: wav_data.to_vec(),
                    format: AudioFormat::Wav, // 实际返回 WAV
                    duration_ms: info.duration_ms,
                    sample_rate: info.sample_rate,
                    channels: info.channels,
                    original_size,
                    transcoded_size: original_size,
                })
            }
        }
    }

    fn get_audio_info(&self, wav_data: &[u8]) -> Result<AudioInfo, TranscodeError> {
        let header = self.parse_wav_header(wav_data)?;

        // 计算时长
        let samples_per_channel = if header.fmt.bits_per_sample > 0 && header.fmt.num_channels > 0 {
            header.data_size
                / (header.fmt.bits_per_sample as usize / 8)
                / header.fmt.num_channels as usize
        } else {
            0
        };

        let duration_ms = if header.fmt.sample_rate > 0 {
            (samples_per_channel as u64 * 1000) / header.fmt.sample_rate as u64
        } else {
            0
        };

        Ok(AudioInfo {
            duration_ms,
            sample_rate: header.fmt.sample_rate,
            channels: header.fmt.num_channels as u8,
            bits_per_sample: header.fmt.bits_per_sample,
            data_size: header.data_size,
        })
    }

    fn supports_format(&self, format: AudioFormat) -> bool {
        match format {
            AudioFormat::Wav => true,
            AudioFormat::Opus => true,
            AudioFormat::Mp3 => false, // TODO: 实现后改为 true
        }
    }
}

impl Default for WavTranscoder {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_wav() -> Vec<u8> {
        // 创建一个简单的 WAV 文件：1秒，16kHz，单声道，16位
        let sample_rate: u32 = 16000;
        let num_channels: u16 = 1;
        let bits_per_sample: u16 = 16;
        let num_samples = sample_rate as usize;

        let data_size = num_samples * (bits_per_sample as usize / 8) * num_channels as usize;
        let file_size = 36 + data_size;

        let mut wav = Vec::with_capacity(44 + data_size);

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(file_size as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // fmt chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&num_channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * num_channels as u32 * (bits_per_sample / 8) as u32;
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        let block_align = num_channels * (bits_per_sample / 8);
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());

        // data chunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data_size as u32).to_le_bytes());

        // 生成静音数据
        for _ in 0..num_samples {
            wav.extend_from_slice(&0i16.to_le_bytes());
        }

        wav
    }

    #[test]
    fn test_parse_wav_header() {
        let transcoder = WavTranscoder::new(true);
        let wav = create_test_wav();

        let info = transcoder.get_audio_info(&wav).unwrap();
        assert_eq!(info.sample_rate, 16000);
        assert_eq!(info.channels, 1);
        assert_eq!(info.bits_per_sample, 16);
        assert!(info.duration_ms >= 990 && info.duration_ms <= 1010); // ~1000ms
    }

    #[tokio::test]
    async fn test_transcode_passthrough() {
        let transcoder = WavTranscoder::new(true);
        let wav = create_test_wav();

        let config = TranscodeConfig {
            format: AudioFormat::Wav,
            ..Default::default()
        };

        let result = transcoder.transcode(&wav, &config).await.unwrap();
        assert_eq!(result.format, AudioFormat::Wav);
        assert_eq!(result.audio_data.len(), wav.len());
    }

    #[test]
    fn test_supports_format() {
        let transcoder = WavTranscoder::new(true);
        assert!(transcoder.supports_format(AudioFormat::Wav));
        assert!(transcoder.supports_format(AudioFormat::Opus));
        // MP3 暂未实现
        assert!(!transcoder.supports_format(AudioFormat::Mp3));
    }

    #[tokio::test]
    async fn test_transcode_to_opus() {
        let transcoder = WavTranscoder::new(true);
        let wav = create_test_wav();

        let config = TranscodeConfig {
            format: AudioFormat::Opus,
            bitrate: Some(32000),
            ..Default::default()
        };

        let result = transcoder.transcode(&wav, &config).await.unwrap();
        assert_eq!(result.format, AudioFormat::Opus);
        // Opus 应该比 WAV 小很多
        assert!(result.transcoded_size < result.original_size);
        // 验证 OGG 头
        assert_eq!(&result.audio_data[0..4], b"OggS");
    }
}
