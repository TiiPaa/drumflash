use nih_plug::params::persist::PersistentField;
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::sync::Arc;

pub struct InstrumentSettingsState {
    pub frequency: AtomicU32,
    pub decay: AtomicU32,
    pub volume: AtomicU32,
    pub filter_freq: AtomicU32,
}

impl InstrumentSettingsState {
    pub fn new(frequency: f32, decay: f32, volume: f32, filter_freq: f32) -> Self {
        Self {
            frequency: AtomicU32::new(frequency.to_bits()),
            decay: AtomicU32::new(decay.to_bits()),
            volume: AtomicU32::new(volume.to_bits()),
            filter_freq: AtomicU32::new(filter_freq.to_bits()),
        }
    }

    pub fn load(&self) -> (f32, f32, f32, f32) {
        (
            f32::from_bits(self.frequency.load(Ordering::Relaxed)),
            f32::from_bits(self.decay.load(Ordering::Relaxed)),
            f32::from_bits(self.volume.load(Ordering::Relaxed)),
            f32::from_bits(self.filter_freq.load(Ordering::Relaxed)),
        )
    }

    pub fn store(&self, frequency: f32, decay: f32, volume: f32, filter_freq: f32) {
        self.frequency.store(frequency.to_bits(), Ordering::Relaxed);
        self.decay.store(decay.to_bits(), Ordering::Relaxed);
        self.volume.store(volume.to_bits(), Ordering::Relaxed);
        self.filter_freq.store(filter_freq.to_bits(), Ordering::Relaxed);
    }
}

pub struct SoundSettingsState {
    pub instruments: [InstrumentSettingsState; 10],
    pub version: AtomicU64,
}

impl SoundSettingsState {
    pub fn new() -> Arc<Self> {
        let defaults = [
            (60.0, 0.5, 0.8, 100.0),     // Kick
            (200.0, 0.47, 0.6, 1000.0),  // Snare
            (8000.0, 0.36, 0.3, 10000.0), // HiHat
            (6000.0, 0.66, 0.4, 8000.0),  // Open HH
            (300.0, 0.3, 0.5, 2000.0),   // Tom1
            (200.0, 0.4, 0.5, 1500.0),   // Tom2
            (120.0, 0.5, 0.5, 1000.0),   // Tom3
            (1200.0, 0.15, 0.7, 2500.0), // Clap
            (8000.0, 1.2, 0.35, 10000.0), // Ride
            (6000.0, 2.0, 0.4, 8000.0),  // Cymbal
        ];

        Arc::new(Self {
            instruments: std::array::from_fn(|i| {
                let (f, d, v, fl) = defaults[i];
                InstrumentSettingsState::new(f, d, v, fl)
            }),
            version: AtomicU64::new(0),
        })
    }

    pub fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    pub fn read_all(&self) -> Vec<f32> {
        let mut result = vec![0.0f32; 40];
        for (i, inst) in self.instruments.iter().enumerate() {
            let (f, d, v, fl) = inst.load();
            result[i * 4] = f;
            result[i * 4 + 1] = d;
            result[i * 4 + 2] = v;
            result[i * 4 + 3] = fl;
        }
        result
    }

    pub fn write_all(&self, values: &[f32]) {
        for (i, inst) in self.instruments.iter().enumerate() {
            inst.store(
                values.get(i * 4).copied().unwrap_or(0.0),
                values.get(i * 4 + 1).copied().unwrap_or(0.0),
                values.get(i * 4 + 2).copied().unwrap_or(0.0),
                values.get(i * 4 + 3).copied().unwrap_or(0.0),
            );
        }
        self.bump_version();
    }
}

#[derive(Clone)]
pub struct PersistentSoundSettings {
    pub state: Arc<SoundSettingsState>,
}

impl PersistentSoundSettings {
    pub fn new() -> Self {
        Self {
            state: SoundSettingsState::new(),
        }
    }
}

impl<'a> PersistentField<'a, Vec<f32>> for PersistentSoundSettings {
    fn set(&self, new_value: Vec<f32>) {
        self.state.write_all(&new_value);
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&Vec<f32>) -> R,
    {
        let values = self.state.read_all();
        f(&values)
    }
}
