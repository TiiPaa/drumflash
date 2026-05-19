//! MIDI file export (SMF Type 0)

use crate::{instrument_registry::INSTRUMENTS, sequencer::SharedPattern};
use std::path::Path;

const TICKS_PER_QUARTER: u32 = 480;
const TICKS_PER_STEP: u32 = TICKS_PER_QUARTER / 4;

fn encode_vlq(mut value: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        bytes.push((value & 0x7F) as u8);
        value >>= 7;
        if value == 0 {
            break;
        }
    }
    for byte in bytes.iter_mut().skip(1) {
        *byte |= 0x80;
    }
    bytes.reverse();
    bytes
}

fn midi_header(track_length: u32) -> Vec<u8> {
    let mut data = Vec::new();
    // MThd
    data.extend_from_slice(b"MThd");
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x06]); // chunk length = 6
    data.extend_from_slice(&[0x00, 0x00]); // format 0
    data.extend_from_slice(&[0x00, 0x01]); // 1 track
    data.extend_from_slice(&[0x01, 0xE0]); // 480 ticks per quarter
    // MTrk
    data.extend_from_slice(b"MTrk");
    data.extend_from_slice(&[
        (track_length >> 24) as u8,
        (track_length >> 16) as u8,
        (track_length >> 8) as u8,
        track_length as u8,
    ]);
    data
}

pub fn export_pattern_to_midi(
    pattern: &SharedPattern,
    bpm: f32,
    path: &Path,
) -> std::io::Result<()> {
    std::fs::write(path, export_pattern_to_midi_data(pattern, bpm))
}

fn export_pattern_to_midi_data(pattern: &SharedPattern, bpm: f32) -> Vec<u8> {
    let microseconds_per_quarter = (60_000_000.0 / bpm).round() as u32;

    let mut events: Vec<(u32, Vec<u8>)> = Vec::new();

    // Set tempo (meta event)
    events.push((
        0,
        vec![
            0xFF,
            0x51,
            0x03,
            (microseconds_per_quarter >> 16) as u8,
            (microseconds_per_quarter >> 8) as u8,
            microseconds_per_quarter as u8,
        ],
    ));

    for step in 0..16 {
        let mask = pattern.load_step_mask(step);
        for (instrument, def) in INSTRUMENTS.iter().enumerate() {
            if (mask & (1u16 << instrument)) != 0 {
                let tick = step as u32 * TICKS_PER_STEP;
                events.push((tick, vec![0x99, def.midi_note, 100]));
                events.push((tick + 10, vec![0x89, def.midi_note, 0]));
            }
        }
    }

    // Sort by absolute tick
    events.sort_by_key(|e| e.0);

    // Convert to delta times
    let mut track_data = Vec::new();
    let mut last_tick = 0u32;
    for (tick, data) in events {
        let delta = tick - last_tick;
        last_tick = tick;
        track_data.extend_from_slice(&encode_vlq(delta));
        track_data.extend_from_slice(&data);
    }

    // End of track
    track_data.extend_from_slice(&encode_vlq(0));
    track_data.extend_from_slice(&[0xFF, 0x2F, 0x00]);

    let mut file_data = midi_header(track_data.len() as u32);
    file_data.extend_from_slice(&track_data);
    file_data
}

/// Export pattern to MIDI bytes in memory (for drag-and-drop).
pub fn export_pattern_to_midi_bytes(pattern: &SharedPattern, bpm: f32) -> std::io::Result<Vec<u8>> {
    Ok(export_pattern_to_midi_data(pattern, bpm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequencer::pattern::Pattern;

    #[test]
    fn midi_export_includes_zap_thirteenth_instrument() {
        let pattern = SharedPattern::new(&Pattern::empty());
        let zap_index = INSTRUMENTS.len() - 1;
        pattern.set_step_mask(0, 1u16 << zap_index);

        let bytes =
            export_pattern_to_midi_bytes(&pattern, 120.0).expect("MIDI export should succeed");

        assert!(
            bytes.windows(3).any(|window| window == [0x99, 37, 100]),
            "Zap note-on event should be exported"
        );
        assert!(
            bytes.windows(3).any(|window| window == [0x89, 37, 0]),
            "Zap note-off event should be exported"
        );
    }
}
