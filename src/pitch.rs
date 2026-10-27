//! Note frequencies and pitch detection.

use std::f64::consts::PI;
use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

/// Converts a note name such as "F#4" or "Bb3" to its frequency in Hz.
pub fn note_to_freq(note: &str, a4: f64) -> Option<f64> {
    let mut chars = note.chars();
    let mut semitones: i32 = match chars.next()? {
        'C' => -9,
        'D' => -7,
        'E' => -5,
        'F' => -4,
        'G' => -2,
        'A' => 0,
        'B' => 2,
        _ => return None,
    };
    let mut rest = chars.as_str();
    if let Some(r) = rest.strip_prefix('#') {
        semitones += 1;
        rest = r;
    } else if let Some(r) = rest.strip_prefix('b') {
        semitones -= 1;
        rest = r;
    }
    let octave: i32 = rest.parse().ok()?;
    semitones += (octave - 4) * 12;
    Some(a4 * 2f64.powf(semitones as f64 / 12.0))
}

/// How far `freq` is from `target`, in cents (hundredths of a semitone).
pub fn cents_off(freq: f64, target: f64) -> f64 {
    1200.0 * (freq / target).log2()
}

/// Finds the dominant frequency in a block of samples using a zero-padded FFT.
pub struct PitchDetector {
    fft: Arc<dyn Fft<f64>>,
    window: Vec<f64>,
    buffer: Vec<Complex<f64>>,
    sample_rate: f64,
}

impl PitchDetector {
    /// `chunk` is the number of samples analysed at a time. The FFT is
    /// zero-padded to 4x that so peak interpolation is smoother.
    pub fn new(chunk: usize, sample_rate: f64) -> Self {
        let fft_size = chunk * 4;
        // Using blackman window. For more information see
        // http://en.wikipedia.org/wiki/Window_function
        let m = (chunk - 1) as f64;
        let window = (0..chunk)
            .map(|n| {
                let x = n as f64 / m;
                0.42 - 0.5 * (2.0 * PI * x).cos() + 0.08 * (4.0 * PI * x).cos()
            })
            .collect();
        PitchDetector {
            fft: FftPlanner::new().plan_fft_forward(fft_size),
            window,
            buffer: vec![Complex::default(); fft_size],
            sample_rate,
        }
    }

    /// Returns the dominant frequency between `low` and `high` Hz, or `None`
    /// if the input is too quiet to be a plucked string.
    pub fn detect(&mut self, samples: &[f32], low: f64, high: f64, threshold: f64) -> Option<f64> {
        assert_eq!(samples.len(), self.window.len());
        let mean = samples.iter().map(|&s| s as f64).sum::<f64>() / samples.len() as f64;
        let rms = (samples
            .iter()
            .map(|&s| (s as f64 - mean).powi(2))
            .sum::<f64>()
            / samples.len() as f64)
            .sqrt();
        if rms < threshold {
            return None;
        }

        for (i, slot) in self.buffer.iter_mut().enumerate() {
            let value = match samples.get(i) {
                Some(&s) => (s as f64 - mean) * self.window[i],
                None => 0.0,
            };
            *slot = Complex::new(value, 0.0);
        }
        self.fft.process(&mut self.buffer);
        let power = |bin: usize| self.buffer[bin].norm_sqr();

        let bin_hz = self.sample_rate / self.buffer.len() as f64;
        let first = ((low / bin_hz) as usize).max(1);
        let last = ((high / bin_hz) as usize + 1).min(self.buffer.len() / 2 - 1);
        let peak = (first..=last).max_by(|&a, &b| power(a).total_cmp(&power(b)))?;

        // Use quadratic interpolation around the peak (on a log scale)
        let y0 = (power(peak - 1) + 1e-20).ln();
        let y1 = (power(peak) + 1e-20).ln();
        let y2 = (power(peak + 1) + 1e-20).ln();
        let denom = y0 - 2.0 * y1 + y2;
        let offset = if denom != 0.0 {
            0.5 * (y0 - y2) / denom
        } else {
            0.0
        };
        Some((peak as f64 + offset) * bin_hz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 44100.0;
    const CHUNK: usize = 4096;

    /// A synthetic string: a fundamental plus decaying harmonics.
    fn pluck(freq: f64) -> Vec<f32> {
        (0..CHUNK)
            .map(|n| {
                let t = n as f64 / RATE;
                let wave: f64 = [1.0, 0.8, 0.5, 0.3]
                    .iter()
                    .enumerate()
                    .map(|(i, a)| a * (2.0 * PI * freq * (i + 1) as f64 * t).sin())
                    .sum();
                (0.1 * wave) as f32
            })
            .collect()
    }

    #[test]
    fn notes_have_the_right_frequency() {
        for (note, freq) in [
            ("A4", 440.0),
            ("C4", 261.63),
            ("G3", 196.0),
            ("F#4", 369.99),
            ("Bb3", 233.08),
        ] {
            assert!(
                (note_to_freq(note, 440.0).unwrap() - freq).abs() < 0.01,
                "{note}"
            );
        }
        assert_eq!(note_to_freq("H4", 440.0), None);
        assert_eq!(note_to_freq("A", 440.0), None);
    }

    #[test]
    fn detects_detuned_strings_in_every_tuning() {
        let mut detector = PitchDetector::new(CHUNK, RATE);
        for tuning in crate::Tuning::ALL {
            for note in tuning.notes() {
                let target = note_to_freq(note, 440.0).unwrap();
                for detune in [-30.0, 0.0, 12.0] {
                    let played = target * 2f64.powf(detune / 1200.0);
                    let low = target / 2f64.sqrt();
                    let high = target * 2f64.sqrt();
                    let freq = detector.detect(&pluck(played), low, high, 0.005).unwrap();
                    let cents = cents_off(freq, target);
                    assert!((cents - detune).abs() < 1.5, "{note} {detune}: got {cents}");
                }
            }
        }
    }

    #[test]
    fn silence_is_ignored() {
        let mut detector = PitchDetector::new(CHUNK, RATE);
        assert_eq!(detector.detect(&[0.0; CHUNK], 300.0, 500.0, 0.005), None);
    }
}
