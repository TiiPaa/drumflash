//! MIDI file export (SMF Type 0)

use crate::{
    sequencer::SharedPattern,
    track::{AtomicTrackLayout, MAX_TRACKS},
};
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
    track_layout: &AtomicTrackLayout,
    bpm: f32,
    pattern_length: usize,
    path: &Path,
) -> std::io::Result<()> {
    std::fs::write(
        path,
        export_pattern_to_midi_data(pattern, track_layout, bpm, pattern_length),
    )
}

fn export_pattern_to_midi_data(
    pattern: &SharedPattern,
    track_layout: &AtomicTrackLayout,
    bpm: f32,
    pattern_length: usize,
) -> Vec<u8> {
    let microseconds_per_quarter = (60_000_000.0 / bpm).round() as u32;
    let steps = pattern_length.clamp(1, 64);

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

    for step in 0..steps {
        let mask = pattern.load_step_mask(step);
        for slot in 0..MAX_TRACKS {
            if !track_layout.is_active(slot) {
                continue;
            }
            if (mask & (1u16 << slot)) != 0 {
                let tick = step as u32 * TICKS_PER_STEP;
                let note = track_layout.midi_note_for_slot(slot);
                events.push((tick, vec![0x99, note, 100]));
                events.push((tick + 10, vec![0x89, note, 0]));
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
#[allow(dead_code)]
pub fn export_pattern_to_midi_bytes(
    pattern: &SharedPattern,
    track_layout: &AtomicTrackLayout,
    bpm: f32,
    pattern_length: usize,
) -> std::io::Result<Vec<u8>> {
    Ok(export_pattern_to_midi_data(
        pattern,
        track_layout,
        bpm,
        pattern_length,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        sequencer::pattern::Pattern,
        track::{AtomicTrackLayout, TrackInstrumentKind, TrackLayoutState},
    };

    fn legacy_layout() -> std::sync::Arc<AtomicTrackLayout> {
        AtomicTrackLayout::from_state(&TrackLayoutState::from_legacy_13())
    }

    #[test]
    fn midi_export_uses_slot_midi_note_including_fourteenth_slot() {
        let mut layout_state = TrackLayoutState::default_layout();
        layout_state.slots[13].active = true;
        layout_state.slots[13].kind = TrackInstrumentKind::Kick;
        layout_state.slots[13].midi_note = 99;
        let layout = AtomicTrackLayout::from_state(&layout_state);

        let pattern = SharedPattern::new(&Pattern::empty());
        pattern.set_step_mask(0, 1u16 << 13);

        let bytes = export_pattern_to_midi_bytes(&pattern, &layout, 120.0, 16)
            .expect("MIDI export should succeed");

        assert!(
            bytes.windows(3).any(|window| window == [0x99, 99, 100]),
            "14th slot note-on should use its custom MIDI note"
        );
        assert!(
            bytes.windows(3).any(|window| window == [0x89, 99, 0]),
            "14th slot note-off should use its custom MIDI note"
        );
    }

    #[test]
    fn midi_export_includes_perc1_thirteenth_instrument() {
        let pattern = SharedPattern::new(&Pattern::empty());
        let perc1_slot = 12;
        pattern.set_step_mask(0, 1u16 << perc1_slot);
        let layout = legacy_layout();

        let bytes = export_pattern_to_midi_bytes(&pattern, &layout, 120.0, 16)
            .expect("MIDI export should succeed");

        assert!(
            bytes.windows(3).any(|window| window == [0x99, 37, 100]),
            "Perc1 note-on event should be exported"
        );
        assert!(
            bytes.windows(3).any(|window| window == [0x89, 37, 0]),
            "Perc1 note-off event should be exported"
        );
    }

    #[test]
    fn midi_export_includes_steps_beyond_first_page() {
        let pattern = SharedPattern::new(&Pattern::empty());
        pattern.set_step_mask(32, 1u16 << 0); // Kick at step 32
        pattern.set_step_mask(63, 1u16 << 1); // Snare at step 63
        let layout = legacy_layout();

        let bytes = export_pattern_to_midi_bytes(&pattern, &layout, 120.0, 64)
            .expect("MIDI export should succeed");

        // Kick note = 36, Snare note = 38
        assert!(
            bytes.windows(3).any(|window| window == [0x99, 36, 100]),
            "Kick note-on at step 32 should be exported"
        );
        assert!(
            bytes.windows(3).any(|window| window == [0x99, 38, 100]),
            "Snare note-on at step 63 should be exported"
        );
    }
}
