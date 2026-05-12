//! Pattern generation module with 4 techniques:
//! A) Probabilistic style-based generation
//! B) Markov-chain generation
//! C) Euclidean rhythm generation
//! D) Classic pattern + variation generation

use nih_plug::prelude::*;

pub mod classic;
pub mod euclidean;
pub mod markov;
pub mod probabilistic;
pub mod styles;

pub use styles::Style;

use crate::sequencer::pattern::Pattern;

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GeneratorType {
    #[id = "prob"]
    #[name = "Probabilistic"]
    #[default]
    Probabilistic,
    #[id = "markov"]
    #[name = "Markov"]
    Markov,
    #[id = "euclid"]
    #[name = "Euclidean"]
    Euclidean,
    #[id = "classic"]
    #[name = "Classic"]
    Classic,
}

pub struct GeneratorParams {
    pub generator_type: GeneratorType,
    pub style_primary: Style,
    pub style_secondary: Style,
    pub style_mix: f32,
    pub density: f32,
    pub variation: f32,
}

/// Generate a pattern using the selected technique.
pub fn generate(params: &GeneratorParams, rng: &mut impl FnMut() -> f32) -> Pattern {
    match params.generator_type {
        GeneratorType::Probabilistic => probabilistic::generate(
            params.style_primary,
            params.style_secondary,
            params.style_mix,
            params.density,
            rng,
        ),
        GeneratorType::Markov => markov::generate(
            params.style_primary,
            params.density,
            rng,
        ),
        GeneratorType::Euclidean => {
            // Default rotations: slightly offset per instrument for groove
            let rotations = [0, 0, 0, 2, 4, 6, 8];
            euclidean::generate(params.style_primary, params.density, &rotations, rng)
        }
        GeneratorType::Classic => classic::generate(
            params.style_primary,
            params.density,
            params.variation,
            rng,
        ),
    }
}
