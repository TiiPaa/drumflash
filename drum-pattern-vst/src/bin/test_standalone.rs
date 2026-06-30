#![allow(dead_code)]

#[path = "../groove.rs"]
mod groove;
#[path = "../instrument_registry.rs"]
mod instrument_registry;
#[path = "../track.rs"]
mod track;
#[path = "../sequencer/mod.rs"]
mod sequencer;
#[path = "../synthesis/mod.rs"]
mod synthesis;

use groove::GrooveType;
use sequencer::Sequencer;
use sequencer::{Pattern, SharedPattern};
use synthesis::DrumSynthesizer;

fn main() {
    println!("Flash Drum - Test Standalone");
    println!("==========================================");
    println!();

    let shared_pattern = SharedPattern::new(&Pattern::rock_pattern());
    let mut sequencer = Sequencer::new(shared_pattern);
    let mut synthesizer = DrumSynthesizer::new();
    let sample_rate = 44100.0;

    synthesizer.initialize(sample_rate);
    sequencer.play();

    println!("Kick drum should be playing...");
    println!("Press Ctrl+C to stop");
    println!();

    // Simulate 5 seconds of playback
    let bpm = 120.0;
    let samples_per_second = sample_rate as usize;
    let total_samples = samples_per_second * 5;

    let mut output_buffer = vec![0.0f32; 512];

    for i in 0..(total_samples / 512) {
        for sample in output_buffer.iter_mut() {
            let triggers = sequencer.process_sample(bpm, sample_rate, 0.0, GrooveType::Swing16);

            for (voice_idx, trigger) in triggers.iter().enumerate() {
                if trigger.should_trigger {
                    synthesizer.trigger(voice_idx, trigger.velocity);
                }
            }

            synthesizer.process_sample(sample);
        }

        // Display activity every second
        if i % (samples_per_second / 512) == 0 {
            let second = i / (samples_per_second / 512);
            let max_sample = output_buffer
                .iter()
                .map(|s| s.abs())
                .fold(0.0f32, |a, b| a.max(b));
            println!("Second {}: max amplitude = {:.4}", second, max_sample);
        }
    }

    println!();
    println!("Test complete!");
}
