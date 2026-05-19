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
    pub filter_env_amount: AtomicU32,
    pub filter_env_decay: AtomicU32,
    pub analog: AtomicU32,
    pub stereo: AtomicU32,
}

/// Number of f32 values serialized per instrument in the persisted state.
pub const FIELDS_PER_INSTRUMENT: usize = 12;

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
        filter_env_amount: f32,
        filter_env_decay: f32,
        analog: f32,
        stereo: f32,
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
            filter_env_amount: AtomicU32::new(filter_env_amount.to_bits()),
            filter_env_decay: AtomicU32::new(filter_env_decay.to_bits()),
            analog: AtomicU32::new(analog.to_bits()),
            stereo: AtomicU32::new(stereo.to_bits()),
        }
    }

    pub fn load(&self) -> (f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32) {
        (
            f32::from_bits(self.frequency.load(Ordering::Relaxed)),
            f32::from_bits(self.decay.load(Ordering::Relaxed)),
            f32::from_bits(self.volume.load(Ordering::Relaxed)),
            f32::from_bits(self.filter_freq.load(Ordering::Relaxed)),
            f32::from_bits(self.release.load(Ordering::Relaxed)),
            f32::from_bits(self.decay_curve.load(Ordering::Relaxed)),
            f32::from_bits(self.release_curve.load(Ordering::Relaxed)),
            f32::from_bits(self.hold.load(Ordering::Relaxed)),
            f32::from_bits(self.filter_env_amount.load(Ordering::Relaxed)),
            f32::from_bits(self.filter_env_decay.load(Ordering::Relaxed)),
            f32::from_bits(self.analog.load(Ordering::Relaxed)),
            f32::from_bits(self.stereo.load(Ordering::Relaxed)),
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
        filter_env_amount: f32,
        filter_env_decay: f32,
        analog: f32,
        stereo: f32,
    ) {
        self.frequency.store(frequency.to_bits(), Ordering::Relaxed);
        self.decay.store(decay.to_bits(), Ordering::Relaxed);
        self.volume.store(volume.to_bits(), Ordering::Relaxed);
        self.filter_freq.store(filter_freq.to_bits(), Ordering::Relaxed);
        self.release.store(release.to_bits(), Ordering::Relaxed);
        self.decay_curve.store(decay_curve.to_bits(), Ordering::Relaxed);
        self.release_curve.store(release_curve.to_bits(), Ordering::Relaxed);
        self.hold.store(hold.to_bits(), Ordering::Relaxed);
        self.filter_env_amount.store(filter_env_amount.to_bits(), Ordering::Relaxed);
        self.filter_env_decay.store(filter_env_decay.to_bits(), Ordering::Relaxed);
        self.analog.store(analog.to_bits(), Ordering::Relaxed);
        self.stereo.store(stereo.to_bits(), Ordering::Relaxed);
    }
}

pub struct SoundSettingsState {
    pub instruments: [InstrumentSettingsState; 12],
    pub version: AtomicU64,
}

impl SoundSettingsState {
    pub fn new() -> Arc<Self> {
        let defaults: [[f32; 12]; 12] = std::array::from_fn(|i| {
            crate::instrument_registry::INSTRUMENTS[i].sound_settings_default
        });

        Arc::new(Self {
            instruments: std::array::from_fn(|i| {
                let [f, d, v, fl, r, dc, rc, h, fea, fed, a, s] = defaults[i];
                InstrumentSettingsState::new(f, d, v, fl, r, dc, rc, h, fea, fed, a, s)
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
            let (f, d, v, fl, r, dc, rc, h, fea, fed, a, s) = inst.load();
            let base = i * FIELDS_PER_INSTRUMENT;
            result[base] = f;
            result[base + 1] = d;
            result[base + 2] = v;
            result[base + 3] = fl;
            result[base + 4] = r;
            result[base + 5] = dc;
            result[base + 6] = rc;
            result[base + 7] = h;
            result[base + 8] = fea;
            result[base + 9] = fed;
            result[base + 10] = a;
            result[base + 11] = s;
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
                values.get(base + 8).copied().unwrap_or(0.0),
                values.get(base + 9).copied().unwrap_or(0.0),
                values.get(base + 10).copied().unwrap_or(1.0),
                values.get(base + 11).copied().unwrap_or(0.0),
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
