use roland_kick_rust::{Kick808Params, Kick808Voice};
use std::fs::{create_dir_all, File};
use std::io::{Result, Write};
use std::path::Path;

fn main() -> Result<()> {
    let sample_rate = 48_000.0;
    let total_samples = (sample_rate as usize) * 2;

    let mut voice = Kick808Voice::new(sample_rate);
    let params = Kick808Params::default();

    // Retrigs volontairement serrés pour tester le comportement.
    let triggers = [0usize, 7_200, 9_600, 9_840, 14_400, 14_640, 19_200, 28_800, 29_040];
    let mut trigger_index = 0usize;

    let mut buffer = vec![0.0f32; total_samples];
    for n in 0..total_samples {
        if trigger_index < triggers.len() && triggers[trigger_index] == n {
            voice.trigger(1.0, &params);
            trigger_index += 1;
        }
        buffer[n] = voice.process(&params);
    }

    let out_dir = Path::new("target");
    create_dir_all(out_dir)?;
    let wav_path = out_dir.join("demo808.wav");
    write_wav_mono_i16(&wav_path, sample_rate as u32, &buffer)?;

    let peak = buffer
        .iter()
        .fold(0.0f32, |acc, &x| acc.max(x.abs()));
    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("WAV écrit dans {:?}", wav_path);
    println!("peak={:.4} rms={:.4}", peak, rms);

    Ok(())
}

fn write_wav_mono_i16(path: &Path, sample_rate: u32, samples: &[f32]) -> Result<()> {
    let mut file = File::create(path)?;

    let data_bytes = (samples.len() * 2) as u32;
    let riff_chunk_size = 36 + data_bytes;
    let byte_rate = sample_rate * 2;
    let block_align = 2u16;
    let bits_per_sample = 16u16;

    file.write_all(b"RIFF")?;
    file.write_all(&riff_chunk_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // PCM fmt chunk size
    file.write_all(&1u16.to_le_bytes())?; // PCM
    file.write_all(&1u16.to_le_bytes())?; // mono
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;

    file.write_all(b"data")?;
    file.write_all(&data_bytes.to_le_bytes())?;

    for &sample in samples {
        let s = sample.clamp(-1.0, 1.0);
        let pcm = (s * i16::MAX as f32) as i16;
        file.write_all(&pcm.to_le_bytes())?;
    }

    Ok(())
}
