//! Hand clap synthesizer.
//!
//! Architecture: layered noise bursts shaped to emulate the slap + room
//! resonance of a real clap.
//! - One short broadband transient ("snap") at the very first burst — the
//!   sound of the palms striking
//! - 4 mid-focused noise bursts fired ~4 ms apart — the layered echoes
//! - Tight bandpass (HP + LP coupled, centre roughly 1.5–2 kHz on default
//!   settings) — this is the difference between a clap and a hi-hat
//! - Exponential decay envelope per burst, with release tail for the room

use super::{dsp, settings::clap::ClapSettings, Voice, VoiceSettings};

/// Lowpass cutoff for the first burst, expressed as a multiple of the highpass
/// cutoff. 2.5 gives a roughly 1.3-octave bandpass — wide enough for body,
/// narrow enough to stay distinct from a hi-hat.
const LP_RATIO: f32 = 2.5;

/// Burst timing in milliseconds since trigger. Non-uniform spacing breaks the
/// metronomic feel and emulates the natural diffusion of palms striking and
/// rebounding off a nearby surface. Default values are intentionally wide so
/// the Echo slider has perceptible range; multiplied by the user's echo
/// amount at runtime.
const BURST_TIMES_MS: [f32; 4] = [0.0, 10.0, 25.0, 50.0];

/// LP cutoff multipliers applied burst-by-burst, relative to the first burst's
/// LP. Each successive echo loses high-frequency content — emulates the
/// absorption that gives a clap its "diffuse / small room" character without a
/// real reverb.
const BURST_LP_RATIOS: [f32; 4] = [1.0, 0.85, 0.72, 0.60];

/// Highpass cutoff for the snap transient, in Hz. Removes the sub-mid energy
/// from the raw noise burst so the snap reads as "paper / dry slap" rather
/// than "broadband whoosh".
const SNAP_HP_HZ: f32 = 3500.0;

pub struct ClapVoice {
    settings: ClapSettings,
    sample_rate: f32,

    noise: dsp::WhiteNoise,
    noise_r: dsp::WhiteNoise,
    filter_hp: dsp::OnePoleFilter,
    filter_hp_r: dsp::OnePoleFilter,
    filter_lp: dsp::OnePoleFilter,
    filter_lp_r: dsp::OnePoleFilter,
    amp_env: dsp::DecayReleaseEnvelope,
    /// Short broadband transient layered on top of the first burst — provides
    /// the palms-strike "snap" that distinguishes a clap from filtered noise.
    snap: dsp::ClickGenerator,
    snap_r: dsp::ClickGenerator,
    /// Highpass filter on the snap output — shifts its character toward the
    /// "paper / dry slap" end of the spectrum.
    snap_hp: dsp::OnePoleFilter,
    snap_hp_r: dsp::OnePoleFilter,

    burst_count: usize,
    samples_since_trigger: usize,
    active: bool,
}

impl ClapVoice {
    pub fn new(sample_rate: f32, settings: ClapSettings) -> Self {
        let hp = settings.filter_freq.max(400.0);
        let lp_base = (hp * LP_RATIO).min(sample_rate * 0.45);

        let mut filter_hp = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter_hp.set_cutoff(hp, sample_rate);
        let mut filter_hp_r = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter_hp_r.set_cutoff(hp, sample_rate);

        let mut filter_lp = dsp::OnePoleFilter::new(dsp::FilterMode::LowPass);
        filter_lp.set_cutoff(lp_base, sample_rate);
        let mut filter_lp_r = dsp::OnePoleFilter::new(dsp::FilterMode::LowPass);
        filter_lp_r.set_cutoff(lp_base, sample_rate);

        let mut snap_hp = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        snap_hp.set_cutoff(SNAP_HP_HZ.min(sample_rate * 0.45), sample_rate);
        let mut snap_hp_r = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        snap_hp_r.set_cutoff(SNAP_HP_HZ.min(sample_rate * 0.45), sample_rate);

        Self {
            settings,
            sample_rate,
            noise: dsp::WhiteNoise::new(0xBADC0FFE),
            noise_r: dsp::WhiteNoise::new(0xDEAD_C0DE),
            filter_hp,
            filter_hp_r,
            filter_lp,
            filter_lp_r,
            amp_env: dsp::DecayReleaseEnvelope::new(
                sample_rate,
                settings.decay_curve,
                settings.decay,
                settings.release_curve,
                settings.release,
            )
            .with_attack_ms(settings.attack * 1000.0),
            // 2 ms decay, mostly noise, moderate level — short snap that gets
            // HP-filtered to the "paper" character.
            snap: dsp::ClickGenerator::new(sample_rate, 2.0, 0.9, 0.6),
            snap_r: dsp::ClickGenerator::new(sample_rate, 2.0, 0.9, 0.6),
            snap_hp,
            snap_hp_r,
            burst_count: 0,
            samples_since_trigger: 0,
            active: false,
        }
    }

    /// Echo amount, scaled into [0, 3]. 0 collapses all bursts to a single
    /// impact (no audible echo); 1 spreads bursts over 50 ms (default clap);
    /// higher values stretch the bursts up to 150 ms apart for a clap-echo.
    fn echo_amount(&self) -> f32 {
        self.settings.echo.clamp(0.0, 3.0)
    }

    fn lp_for_burst(&self, burst_idx: usize) -> f32 {
        let hp = self.settings.filter_freq.max(400.0);
        let lp_base = (hp * LP_RATIO).min(self.sample_rate * 0.45);
        let default_ratio = BURST_LP_RATIOS[burst_idx.min(BURST_LP_RATIOS.len() - 1)];
        // Interpolate the LP ratio toward 1.0 (no shift) as echo → 0.
        let echo = self.echo_amount();
        let ratio = 1.0 - (1.0 - default_ratio) * echo;
        (lp_base * ratio).max(hp * 1.1)
    }

    fn burst_time_samples(&self, burst_idx: usize) -> usize {
        // Scale the configured burst times by the echo amount. At echo=0 all
        // bursts collapse to sample 0 and read as a single hit.
        let ms = if self.settings.algo == 1 {
            // Tight: much shorter spacing for a compact slap
            let tight_times = [0.0f32, 3.0, 7.0, 12.0];
            tight_times[burst_idx.min(3)] * self.echo_amount()
        } else {
            BURST_TIMES_MS[burst_idx.min(BURST_TIMES_MS.len() - 1)] * self.echo_amount()
        };
        (ms / 1000.0 * self.sample_rate) as usize
    }

    fn update_derived_params(&mut self) {
        let hp = self.settings.filter_freq.max(400.0);
        self.filter_hp.set_cutoff(hp, self.sample_rate);
        self.filter_hp_r.set_cutoff(hp, self.sample_rate);
        // LP is set per-burst (in process_sample) so the timbre evolves; here
        // we just reset to the first-burst value when settings change.
        let lp = self.lp_for_burst(0);
        self.filter_lp.set_cutoff(lp, self.sample_rate);
        self.filter_lp_r.set_cutoff(lp, self.sample_rate);
        self.amp_env.set_decay(self.settings.decay);
        self.amp_env.set_attack_ms(self.settings.attack * 1000.0);
        self.amp_env.set_release(self.settings.release);
        self.amp_env.set_decay_curve(self.settings.decay_curve);
        self.amp_env.set_release_curve(self.settings.release_curve);
    }
}

impl Voice for ClapVoice {
    fn trigger(&mut self) {
        self.active = true;
        self.burst_count = 0;
        self.samples_since_trigger = 0;
        // Keep filters and noise generator continuous across triggers.
        // amp_env and snap are triggered per-burst in process_sample so each
        // echo gets its own audible transient instead of fading into the tail
        // of the first hit.
        self.amp_env.trigger();
        self.snap.trigger();
        self.snap_r.trigger();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Fire each burst at its echo-scaled sample offset, shifting the LP
        // cutoff down for each echo so the timbre diffuses. Re-trigger the
        // snap at every burst (not just the first one) so each echo gets its
        // own audible transient — without this, echoes blend into the first
        // hit's tail and are not perceived as separate impacts.
        if self.burst_count < 4
            && self.samples_since_trigger >= self.burst_time_samples(self.burst_count)
        {
            self.amp_env.trigger();
            // Don't re-trigger the snap on the very first burst — it was
            // already fired in the main trigger() so its envelope is fresh.
            if self.burst_count > 0 {
                self.snap.trigger();
                self.snap_r.trigger();
            }
            let lp = self.lp_for_burst(self.burst_count);
            self.filter_lp.set_cutoff(lp, self.sample_rate);
            self.filter_lp_r.set_cutoff(lp, self.sample_rate);
            self.burst_count += 1;
        }
        self.samples_since_trigger += 1;

        // Each successive echo is a bit quieter than the previous one (-18 %),
        // keeping echoes clearly audible even when the user pushes Echo high.
        let burst_intensity = 1.0 - ((self.burst_count.saturating_sub(1)) as f32 * 0.18);

        let env = self.amp_env.next();
        if env <= 0.0 && self.burst_count >= 4 && !self.snap.is_active() {
            self.active = false;
            return 0.0;
        }

        let noise = self.noise.next();
        let hp = self.filter_hp.process(noise);
        let lp = self.filter_lp.process(hp);
        let body = lp * env * burst_intensity * self.settings.volume;

        // Snap transient: highpassed to keep only the "paper / dry slap" band.
        // Scaled by burst_intensity so echo-snaps are quieter than the main hit.
        let snap_signal = if self.snap.is_active() {
            self.snap_hp.process(self.snap.next()) * self.settings.volume * burst_intensity
        } else {
            0.0
        };

        body + snap_signal
    }

    fn process_sample_stereo(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        if self.settings.stereo < 0.5 {
            let m = self.process_sample();
            return (m, m);
        }

        // Same burst logic as mono
        if self.burst_count < 4
            && self.samples_since_trigger >= self.burst_time_samples(self.burst_count)
        {
            self.amp_env.trigger();
            if self.burst_count > 0 {
                self.snap.trigger();
                self.snap_r.trigger();
            }
            let lp = self.lp_for_burst(self.burst_count);
            self.filter_lp.set_cutoff(lp, self.sample_rate);
            self.filter_lp_r.set_cutoff(lp, self.sample_rate);
            self.burst_count += 1;
        }
        self.samples_since_trigger += 1;

        let burst_intensity = 1.0 - ((self.burst_count.saturating_sub(1)) as f32 * 0.18);
        let env = self.amp_env.next();
        if env <= 0.0 && self.burst_count >= 4 && !self.snap.is_active() {
            self.active = false;
            return (0.0, 0.0);
        }

        let noise_l = self.noise.next();
        let noise_r = self.noise_r.next();
        let hp_l = self.filter_hp.process(noise_l);
        let hp_r = self.filter_hp_r.process(noise_r);
        let lp_l = self.filter_lp.process(hp_l);
        let lp_r = self.filter_lp_r.process(hp_r);
        let body_l = lp_l * env * burst_intensity * self.settings.volume;
        let body_r = lp_r * env * burst_intensity * self.settings.volume;

        let snap_l = if self.snap.is_active() {
            self.snap_hp.process(self.snap.next()) * self.settings.volume * burst_intensity
        } else {
            0.0
        };
        let snap_r = if self.snap_r.is_active() {
            self.snap_hp_r.process(self.snap_r.next()) * self.settings.volume * burst_intensity
        } else {
            0.0
        };

        (body_l + snap_l, body_r + snap_r)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.amp_env.reset();
        self.snap.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = ClapSettings::from(settings);
        self.update_derived_params();
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.settings.echo = value;
        }
    }
}
