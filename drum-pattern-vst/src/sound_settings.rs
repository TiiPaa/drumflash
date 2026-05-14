use nih_plug::params::persist::PersistentField;
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::sync::Arc;

/// Per-voice persistent sound settings. The amplitude envelope is bi-stage
/// (decay + release), each stage having an independent time and curve, plus
/// a hold phase between attack and decay (snare/HH use it for sustain).
pub struct InstrumentSettingsState {
    pub frequency: AtomicU32,
    pub decay: AtomicU32,
    pub volume: AtomicU32,
    pub filter_freq: AtomicU32,
    pub release: AtomicU32,
    pub decay_curve: AtomicU32,
    pub release_curve: AtomicU32,
    pub hold: AtomicU32,
}

/// Number of f32 values serialized per instrument in the persisted state.
pub const FIELDS_PER_INSTRUMENT: usize = 8;

impl InstrumentSettingsState {
    pub fn new(
        frequency: f32,
        decay: f32,
        volume: f32,
        filter_freq: f32,
        release: f32,
        decay_curve: f32,
        release_curve: f32,
        hold: f32,
    ) -> Self {
        Self {
            frequency: AtomicU32::new(frequency.to_bits()),
            decay: AtomicU32::new(decay.to_bits()),
            volume: AtomicU32::new(volume.to_bits()),
            filter_freq: AtomicU32::new(filter_freq.to_bits()),
            release: AtomicU32::new(release.to_bits()),
            decay_curve: AtomicU32::new(decay_curve.to_bits()),
            release_curve: AtomicU32::new(release_curve.to_bits()),
            hold: AtomicU32::new(hold.to_bits()),
        }
    }

    pub fn load(&self) -> (f32, f32, f32, f32, f32, f32, f32, f32) {
        (
            f32::from_bits(self.frequency.load(Ordering::Relaxed)),
            f32::from_bits(self.decay.load(Ordering::Relaxed)),
            f32::from_bits(self.volume.load(Ordering::Relaxed)),
            f32::from_bits(self.filter_freq.load(Ordering::Relaxed)),
            f32::from_bits(self.release.load(Ordering::Relaxed)),
            f32::from_bits(self.decay_curve.load(Ordering::Relaxed)),
            f32::from_bits(self.release_curve.load(Ordering::Relaxed)),
            f32::from_bits(self.hold.load(Ordering::Relaxed)),
        )
    }

    pub fn store(
        &self,
        frequency: f32,
        decay: f32,
        volume: f32,
        filter_freq: f32,
        release: f32,
        decay_curve: f32,
        release_curve: f32,
        hold: f32,
    ) {
        self.frequency.store(frequency.to_bits(), Ordering::Relaxed);
        self.decay.store(decay.to_bits(), Ordering::Relaxed);
        self.volume.store(volume.to_bits(), Ordering::Relaxed);
        self.filter_freq.store(filter_freq.to_bits(), Ordering::Relaxed);
        self.release.store(release.to_bits(), Ordering::Relaxed);
        self.decay_curve.store(decay_curve.to_bits(), Ordering::Relaxed);
        self.release_curve.store(release_curve.to_bits(), Ordering::Relaxed);
        self.hold.store(hold.to_bits(), Ordering::Relaxed);
    }
}

pub struct SoundSettingsState {
    pub instruments: [InstrumentSettingsState; 11],
    pub version: AtomicU64,
}

impl SoundSettingsState {
    pub fn new() -> Arc<Self> {
        // (frequency, decay, volume, filter_freq, release, decay_curve, release_curve, hold)
        let defaults = [
            (60.0,   0.5,  0.8,  100.0,   0.5,  5.0, 3.0, 0.0),  // Kick
            (200.0,  0.47, 0.6,  1000.0,  0.2,  5.0, 3.0, 0.0),  // Snare
            (8000.0, 0.36, 0.3,  10000.0, 0.0,  8.0, 3.0, 0.0),  // HiHat
            (6000.0, 0.66, 0.4,  8000.0,  0.4,  5.5, 3.0, 0.0),  // Open HH
            (300.0,  0.3,  0.5,  2000.0,  0.3,  4.2, 3.0, 0.0),  // Tom1
            (200.0,  0.4,  0.5,  1500.0,  0.4,  4.2, 3.0, 0.0),  // Tom2
            (120.0,  0.5,  0.5,  1000.0,  0.5,  4.2, 3.0, 0.0),  // Tom3
            (1200.0, 0.03, 0.7,  1000.0,  0.12, 6.0, 3.0, 0.0),  // Clap
            (8000.0, 1.2,  0.35, 10000.0, 1.5,  3.5, 3.0, 0.0),  // Ride
            (6000.0, 2.0,  0.4,  8000.0,  2.5,  2.8, 3.0, 0.0),  // Cymbal
            (220.0,  0.08, 0.7,  3000.0,  0.15, 5.0, 3.0, 0.0),  // Snare 606
        ];

        Arc::new(Self {
            instruments: std::array::from_fn(|i| {
                let (f, d, v, fl, r, dc, rc, h) = defaults[i];
                InstrumentSettingsState::new(f, d, v, fl, r, dc, rc, h)
            }),
            version: AtomicU64::new(0),
        })
    }

    pub fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    pub fn read_all(&self) -> Vec<f32> {
        let mut result = vec![0.0f32; self.instruments.len() * FIELDS_PER_INSTRUMENT];
        for (i, inst) in self.instruments.iter().enumerate() {
            let (f, d, v, fl, r, dc, rc, h) = inst.load();
            let base = i * FIELDS_PER_INSTRUMENT;
            result[base] = f;
            result[base + 1] = d;
            result[base + 2] = v;
            result[base + 3] = fl;
            result[base + 4] = r;
            result[base + 5] = dc;
            result[base + 6] = rc;
            result[base + 7] = h;
        }
        result
    }

    pub fn write_all(&self, values: &[f32]) {
        for (i, inst) in self.instruments.iter().enumerate() {
            let base = i * FIELDS_PER_INSTRUMENT;
            inst.store(
                values.get(base).copied().unwrap_or(0.0),
                values.get(base + 1).copied().unwrap_or(0.0),
                values.get(base + 2).copied().unwrap_or(0.0),
                values.get(base + 3).copied().unwrap_or(0.0),
                values.get(base + 4).copied().unwrap_or(0.0),
                values.get(base + 5).copied().unwrap_or(0.0),
                values.get(base + 6).copied().unwrap_or(0.0),
                values.get(base + 7).copied().unwrap_or(0.0),
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
