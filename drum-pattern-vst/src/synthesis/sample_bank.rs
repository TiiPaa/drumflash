//! Embedded multisample banks — 8 hits per TR-606 instrument.
//!
//! Each WAV (float32 mono, 44.1 kHz) holds the 8 hits back to back at even
//! spacing. Files are embedded with `include_bytes!` and decoded once into
//! global `OnceLock`s. Playback handles the session sample-rate conversion
//! through fractional-position interpolation, so no resampling pass is
//! needed at load time.
//!
//! Real-time contract: banks must be pre-warmed from
//! `DrumSynthesizer::initialize_with_layout` (non-RT). After that, calls from
//! the audio thread are a single atomic load — no allocation, no lock.

use std::sync::OnceLock;

pub const HIT_COUNT: usize = 8;

static BD606_BYTES: &[u8] = include_bytes!("../../assets/bd606.wav");
static SD606_BYTES: &[u8] = include_bytes!("../../assets/sd606.wav");
static CH606_BYTES: &[u8] = include_bytes!("../../assets/ch606.wav");

pub struct SampleBank {
    pub source_rate: f32,
    pub hits: [Vec<f32>; HIT_COUNT],
}

static BD606_BANK: OnceLock<SampleBank> = OnceLock::new();
static SD606_BANK: OnceLock<SampleBank> = OnceLock::new();
static CH606_BANK: OnceLock<SampleBank> = OnceLock::new();

/// TR-606 bass drum bank (8 × 1 s hits).
pub fn bd606() -> &'static SampleBank {
    BD606_BANK.get_or_init(|| load_bank(BD606_BYTES))
}

/// TR-606 snare bank (8 × 0.5 s hits).
pub fn sd606() -> &'static SampleBank {
    SD606_BANK.get_or_init(|| load_bank(SD606_BYTES))
}

/// TR-606 closed hi-hat bank (8 × 0.5 s hits).
pub fn ch606() -> &'static SampleBank {
    CH606_BANK.get_or_init(|| load_bank(CH606_BYTES))
}

fn read_u16_le(bytes: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*bytes.get(off)?, *bytes.get(off + 1)?]))
}

fn read_u32_le(bytes: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *bytes.get(off)?,
        *bytes.get(off + 1)?,
        *bytes.get(off + 2)?,
        *bytes.get(off + 3)?,
    ]))
}

fn load_bank(bytes: &[u8]) -> SampleBank {
    let is_riff = bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE";

    let mut source_rate = 44100u32;
    let mut audio_format = 3u16; // IEEE float
    let mut channels = 1u16;
    let mut bits_per_sample = 32u16;
    let mut data: Option<(usize, usize)> = None; // (offset, len)

    if is_riff {
        let mut pos = 12usize;
        while pos + 8 <= bytes.len() {
            let id = &bytes[pos..pos + 4];
            let size = read_u32_le(bytes, pos + 4).unwrap_or(0) as usize;
            let body = pos + 8;
            if body + size > bytes.len() {
                break;
            }
            match id {
                b"fmt " => {
                    audio_format = read_u16_le(bytes, body).unwrap_or(3);
                    channels = read_u16_le(bytes, body + 2).unwrap_or(1);
                    source_rate = read_u32_le(bytes, body + 4).unwrap_or(44100);
                    bits_per_sample = read_u16_le(bytes, body + 14).unwrap_or(32);
                }
                b"data" => {
                    data = Some((body, size));
                    break;
                }
                _ => {}
            }
            pos = body + size + (size & 1);
        }
    }

    let samples: Vec<f32> = match data {
        Some((off, len)) if audio_format == 3 && bits_per_sample == 32 => {
            let frame = 4 * channels.max(1) as usize;
            (0..len / frame)
                .map(|i| {
                    let o = off + i * frame;
                    f32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]])
                })
                .collect()
        }
        Some((off, len)) if audio_format == 1 && bits_per_sample == 16 => {
            let frame = 2 * channels.max(1) as usize;
            (0..len / frame)
                .map(|i| {
                    let o = off + i * frame;
                    i16::from_le_bytes([bytes[o], bytes[o + 1]]) as f32 / 32768.0
                })
                .collect()
        }
        // Unparseable file: fall back to silence so the voice stays inert
        // instead of panicking on host-supplied-adjacent data.
        _ => Vec::new(),
    };

    let hit_len = samples.len() / HIT_COUNT;
    let hits: [Vec<f32>; HIT_COUNT] = std::array::from_fn(|h| {
        if hit_len == 0 {
            Vec::new()
        } else {
            samples[h * hit_len..(h + 1) * hit_len].to_vec()
        }
    });

    SampleBank {
        source_rate: source_rate as f32,
        hits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_bank(bank: &SampleBank, expected_hit_len: usize) {
        assert_eq!(bank.source_rate, 44100.0);
        for (i, hit) in bank.hits.iter().enumerate() {
            assert_eq!(hit.len(), expected_hit_len, "hit {i} length");
            let peak = hit.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            assert!(peak > 0.1, "hit {i} is silent (peak {peak})");
        }
    }

    #[test]
    fn bd606_bank_decodes_eight_nonempty_hits() {
        check_bank(bd606(), 44100);
    }

    #[test]
    fn sd606_bank_decodes_eight_nonempty_hits() {
        check_bank(sd606(), 22050);
    }

    #[test]
    fn each_hit_starts_with_an_attack() {
        for bank in [bd606(), sd606()] {
            for (i, hit) in bank.hits.iter().enumerate() {
                let head = hit[..4410].iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                assert!(head > 0.1, "hit {i} has no attack in its first 100 ms");
            }
        }
    }
}
