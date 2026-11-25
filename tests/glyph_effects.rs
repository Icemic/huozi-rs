use std::f64::consts::PI;
use std::ops::Range;

use huozi::effects::{Effect, EffectContext, GlyphAppearance};

const DEFAULT_GLOW: Effect = Effect::Glow {
    color: [1.0, 0.86, 0.53, 1.0],
};

const EFFECTS: [Effect; 5] = [
    Effect::Cluster,
    Effect::Wave,
    Effect::Shake,
    DEFAULT_GLOW,
    Effect::Glitch,
];

fn context(source: Range<usize>, elapsed_seconds: f64) -> EffectContext {
    EffectContext {
        source,
        elapsed_seconds,
        visible_characters: None,
        reduce_motion: false,
    }
}

fn appearance() -> GlyphAppearance {
    GlyphAppearance {
        offset: [3.0, -4.0],
        color: [0.2, 0.3, 0.4, 0.25],
        visible: true,
    }
}

fn assert_color(actual: [f32; 4], expected: [f32; 4]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.000001,
            "{actual} != {expected}"
        );
    }
}

#[test]
fn names_are_engine_independent_and_unknown_names_are_rejected() {
    for (name, expected) in [
        ("cluster", Effect::Cluster),
        ("wave", Effect::Wave),
        ("shake", Effect::Shake),
        ("glow", DEFAULT_GLOW),
        ("glitch", Effect::Glitch),
    ] {
        assert_eq!(Effect::from_name(name), Some(expected));
    }
    for name in ["", "hz_wave", "Wave", "wave ", "unknown"] {
        assert_eq!(Effect::from_name(name), None);
    }
}

#[test]
fn partial_source_clusters_remain_hidden_until_the_exclusive_end() {
    // 连字、组合字符和 ZWJ 序列使用相同的 scalar 范围约定。
    for source in [0..3, 5..7, 9..16] {
        for effect in EFFECTS {
            let mut input = context(source.clone(), 0.0);
            input.visible_characters = Some(source.end - 1);
            let mut glyph = appearance();
            effect.apply(&input, &mut glyph);
            assert!(!glyph.visible, "{effect:?}: {source:?}");
            assert_eq!(glyph.offset, appearance().offset);
            assert_eq!(glyph.color, appearance().color);

            input.visible_characters = Some(source.end);
            let mut glyph = appearance();
            effect.apply(&input, &mut glyph);
            assert!(glyph.visible, "{effect:?}: {source:?}");
        }
    }
}

#[test]
fn unlimited_reveal_and_cluster_guard_preserve_the_existing_appearance() {
    let input = context(7..12, 42.0);
    let mut glyph = appearance();
    Effect::Cluster.apply(&input, &mut glyph);
    assert!(glyph.visible);
    assert_eq!(glyph.offset, appearance().offset);
    assert_eq!(glyph.color, appearance().color);
}

#[test]
fn every_effect_preserves_alpha_and_never_reveals_an_already_hidden_glyph() {
    for effect in EFFECTS {
        let mut glyph = appearance();
        glyph.visible = false;
        effect.apply(&context(1..2, 0.0), &mut glyph);
        assert!(!glyph.visible, "{effect:?}");
        assert_eq!(glyph.color[3], 0.25, "{effect:?}");
    }
}

#[test]
fn wave_retains_its_fixed_pixel_amplitude_and_time_scale() {
    for (elapsed, expected) in [
        (0.0, [3.0, -4.0]),
        (PI / 6.0, [3.0, -2.0]),
        (PI / 2.0, [3.0, -6.0]),
    ] {
        let mut glyph = appearance();
        Effect::Wave.apply(&context(0..1, elapsed), &mut glyph);
        assert_eq!(glyph.offset, expected);
        assert_eq!(glyph.color, appearance().color);
    }
}

#[test]
fn shake_retains_twelve_hertz_steps_and_adds_to_the_existing_offset() {
    for (elapsed, expected) in [
        (0.0, [3.0, -4.0]),
        (0.1, [4.0, -5.0]),
        (0.15, [4.0, -5.0]),
        (0.2, [4.0, -4.0]),
    ] {
        let mut glyph = appearance();
        Effect::Shake.apply(&context(0..1, elapsed), &mut glyph);
        assert_eq!(glyph.offset, expected);
    }
}

#[test]
fn glitch_is_intermittent_and_blends_from_the_current_color() {
    let mut active = appearance();
    Effect::Glitch.apply(&context(1..2, 0.0), &mut active);
    assert_eq!(active.offset, [5.0, -4.0]);
    assert_color(active.color, [0.41, 0.69, 0.76, 0.25]);

    for input in [context(0..1, 0.0), context(1..2, 0.125)] {
        let mut glyph = appearance();
        Effect::Glitch.apply(&input, &mut glyph);
        assert_eq!(glyph.offset, appearance().offset);
        assert_eq!(glyph.color, appearance().color);
    }
}

#[test]
fn glow_pulses_rgb_without_changing_alpha_or_position() {
    for (elapsed, expected) in [
        (0.0, [0.488, 0.5016, 0.4468, 0.25]),
        (PI / 4.0, [0.52, 0.524, 0.452, 0.25]),
        (3.0 * PI / 4.0, [0.456, 0.4792, 0.4416, 0.25]),
    ] {
        let mut glyph = appearance();
        DEFAULT_GLOW.apply(&context(0..1, elapsed), &mut glyph);
        assert_color(glyph.color, expected);
        assert_eq!(glyph.offset, appearance().offset);
    }
}

#[test]
fn effect_glow_uses_configured_color_and_alpha_as_blend_weight() {
    let effect = Effect::Glow {
        color: [0.1, 0.8, 0.4, 0.5],
    };
    for (elapsed, reduce_motion, expected) in [
        (0.0, true, [0.182, 0.39, 0.4, 0.25]),
        (PI / 4.0, false, [0.18, 0.4, 0.4, 0.25]),
    ] {
        let mut input = context(0..1, elapsed);
        input.reduce_motion = reduce_motion;
        let mut glyph = appearance();
        effect.apply(&input, &mut glyph);
        assert_color(glyph.color, expected);
        assert_eq!(glyph.offset, appearance().offset);
        assert!(glyph.visible);
    }

    let mut transparent = appearance();
    Effect::Glow {
        color: [0.0, 1.0, 0.0, 0.0],
    }
    .apply(&context(0..1, 2.0), &mut transparent);
    assert_eq!(transparent, appearance());
}

#[test]
fn reduced_motion_keeps_only_a_time_independent_glow() {
    for elapsed in [0.0, 0.125, 2.25, 120.0] {
        let mut input = context(1..4, elapsed);
        input.reduce_motion = true;
        for effect in EFFECTS {
            let mut glyph = appearance();
            effect.apply(&input, &mut glyph);
            assert_eq!(glyph.offset, appearance().offset, "{effect:?}");
            assert!(glyph.visible);
            if effect == DEFAULT_GLOW {
                assert_color(glyph.color, [0.488, 0.5016, 0.4468, 0.25]);
            } else {
                assert_eq!(glyph.color, appearance().color, "{effect:?}");
            }
        }
    }
}

#[test]
fn nested_effects_compose_instead_of_overwriting_previous_paint() {
    let mut glyph = appearance();
    let mut input = context(1..2, 0.0);
    input.reduce_motion = true;
    DEFAULT_GLOW.apply(&input, &mut glyph);
    input.reduce_motion = false;
    for effect in [Effect::Wave, Effect::Shake, Effect::Glitch] {
        effect.apply(&input, &mut glyph);
    }
    assert_eq!(glyph.offset, [6.0, -1.0]);
    assert_color(glyph.color, [0.5252, 0.77064, 0.77872, 0.25]);
    assert!(glyph.visible);
}

#[test]
fn the_same_cluster_start_has_the_same_phase_independent_of_cluster_length() {
    for effect in EFFECTS {
        let mut single = appearance();
        let mut cluster = appearance();
        effect.apply(&context(7..8, 2.25), &mut single);
        effect.apply(&context(7..14, 2.25), &mut cluster);
        assert_eq!(single.offset, cluster.offset, "{effect:?}");
        assert_eq!(single.color, cluster.color, "{effect:?}");
    }
}

#[test]
fn repeated_evaluation_is_deterministic_and_motion_stays_bounded() {
    for (effect, bounds) in [
        (Effect::Wave, [0.0, 2.0]),
        (Effect::Shake, [1.0, 1.0]),
        (Effect::Glitch, [2.0, 0.0]),
    ] {
        for source in 0..12 {
            for frame in 0..96 {
                let input = context(source..source + 1, frame as f64 / 24.0);
                let mut first = appearance();
                let mut second = appearance();
                effect.apply(&input, &mut first);
                effect.apply(&input, &mut second);
                assert_eq!(first.offset, second.offset, "{effect:?}");
                assert_eq!(first.color, second.color, "{effect:?}");
                for (axis, bound) in bounds.into_iter().enumerate() {
                    assert!(
                        (first.offset[axis] - appearance().offset[axis]).abs() <= bound,
                        "{effect:?}"
                    );
                }
            }
        }
    }
}
