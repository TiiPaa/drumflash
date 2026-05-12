//! Probabilistic pattern generator (Technique A).
//! Uses structured musical templates with anchor/candidate/exclusion roles.

use super::styles::{generate_from_template, mix_templates, MusicalTemplate, Style};
use crate::sequencer::pattern::Pattern;

pub fn generate(style_a: Style, style_b: Style, mix: f32, density: f32, rng: &mut impl FnMut() -> f32) -> Pattern {
    let template_a = MusicalTemplate::for_style(style_a);
    if mix <= 0.0 {
        generate_from_template(&template_a, density, rng)
    } else {
        let template_b = MusicalTemplate::for_style(style_b);
        let blended = mix_templates(&template_a, &template_b, mix);
        generate_from_template(&blended, density, rng)
    }
}
