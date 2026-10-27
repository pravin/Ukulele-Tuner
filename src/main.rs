//! Ukulele Tuner - A console utility to tune your Ukulele
//!
//! Copyright (C) 2011 Pravin Paratey <pravinp@gmail.com>
//!
//! This program is free software: you can redistribute it and/or modify it
//! under the terms of the GNU General Public License as published by the Free
//! Software Foundation, either version 3 of the License, or (at your option)
//! any later version.
//!
//! This program is distributed in the hope that it will be useful, but WITHOUT
//! ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
//! FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//!
//! You should have received a copy of the GNU General Public License along with
//! this program.  If not, see <http://www.gnu.org/licenses/>

mod pitch;

use std::collections::VecDeque;
use std::io::{stdout, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use crossterm::cursor::{Hide, MoveToColumn, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, Print, Stylize};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{execute, queue};

use pitch::{cents_off, note_to_freq, PitchDetector};

/// Each string is only searched for within half an octave of its target note.
/// This stops strong harmonics (2x, 3x the fundamental) from being picked up.
const SEARCH_SEMITONES: f64 = 6.0;

/// Number of readings to take the median over. This smooths out noise
/// between string strokes and stops the meter jumping around.
const SMOOTHING: usize = 5;

const METER_RANGE: f64 = 50.0; // Cents shown on either side of the meter's centre
const IN_TUNE_CENTS: f64 = 5.0; // Within this many cents counts as in tune (green)
const CLOSE_CENTS: f64 = 20.0; // Within this many cents counts as close (yellow)

#[derive(Clone, Copy, ValueEnum)]
enum Tuning {
    /// GCEA with a high G
    Standard,
    /// GCEA with a low G
    LowG,
    /// ADF#B
    D,
    /// ADF#B with a low A
    LowA,
    /// DGBE
    Baritone,
}

impl Tuning {
    #[cfg(test)]
    const ALL: [Tuning; 5] = [
        Tuning::Standard,
        Tuning::LowG,
        Tuning::D,
        Tuning::LowA,
        Tuning::Baritone,
    ];

    /// Strings are listed from string 4 (nearest your chin) to string 1.
    fn notes(self) -> [&'static str; 4] {
        match self {
            Tuning::Standard => ["G4", "C4", "E4", "A4"],
            Tuning::LowG => ["G3", "C4", "E4", "A4"],
            Tuning::D => ["A4", "D4", "F#4", "B4"],
            Tuning::LowA => ["A3", "D4", "F#4", "B4"],
            Tuning::Baritone => ["D3", "G3", "B3", "E4"],
        }
    }
}

#[derive(Parser)]
#[command(version, about = "Tune your ukulele from the terminal.")]
struct Args {
    /// Tuning to use
    #[arg(short, long, value_enum, default_value_t = Tuning::Standard)]
    tuning: Tuning,

    /// Reference pitch for A4
    #[arg(long, default_value_t = 440.0, value_name = "HZ")]
    a4: f64,

    /// Input level below which sound is ignored
    #[arg(long, default_value_t = 0.005)]
    threshold: f64,

    /// Input device number or part of its name (see --list-devices)
    #[arg(short, long)]
    device: Option<String>,

    /// List audio input devices and exit
    #[arg(long)]
    list_devices: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.list_devices {
        return list_devices();
    }
    let (_stream, audio, sample_rate) = open_input(args.device.as_deref())?;
    let _terminal = RawTerminal::enable()?;
    tune(&args, &audio, sample_rate)
}

fn list_devices() -> Result<()> {
    let host = cpal::default_host();
    let default = host.default_input_device().and_then(|d| d.id().ok());
    for (i, device) in host.input_devices()?.enumerate() {
        let marker = if device.id().ok() == default {
            '*'
        } else {
            ' '
        };
        let name = device
            .description()
            .map(|d| d.to_string())
            .unwrap_or_default();
        println!("{marker} {i:2} {name}");
    }
    Ok(())
}

fn find_device(host: &cpal::Host, wanted: &str) -> Result<cpal::Device> {
    let mut devices = host.input_devices()?;
    let device = match wanted.parse::<usize>() {
        Ok(index) => devices.nth(index),
        Err(_) => devices.find(|d| {
            d.description()
                .map(|desc| {
                    desc.to_string()
                        .to_lowercase()
                        .contains(&wanted.to_lowercase())
                })
                .unwrap_or(false)
        }),
    };
    device.with_context(|| format!("No input device matching {wanted:?} (try --list-devices)"))
}

/// Opens the microphone. Audio arrives on the returned channel as blocks of
/// mono samples; the stream stops when it is dropped.
fn open_input(wanted: Option<&str>) -> Result<(cpal::Stream, Receiver<Vec<f32>>, f64)> {
    let host = cpal::default_host();
    let device = match wanted {
        Some(wanted) => find_device(&host, wanted)?,
        None => host.default_input_device().context("No microphone found")?,
    };
    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate() as f64;
    let (tx, rx) = mpsc::channel();
    let stream = match config.sample_format() {
        SampleFormat::F32 => build_stream::<f32>(&device, config.into(), tx),
        SampleFormat::I16 => build_stream::<i16>(&device, config.into(), tx),
        SampleFormat::I32 => build_stream::<i32>(&device, config.into(), tx),
        SampleFormat::U16 => build_stream::<u16>(&device, config.into(), tx),
        format => bail!("Unsupported sample format {format}"),
    }?;
    stream.play()?;
    Ok((stream, rx, sample_rate))
}

fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    tx: Sender<Vec<f32>>,
) -> Result<cpal::Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            // Only the first channel is used
            let mono = data
                .chunks(channels)
                .map(|frame| frame[0].to_sample::<f32>());
            let _ = tx.send(mono.collect());
        },
        |err| eprint!("\r\nAudio error: {err}\r\n"),
        None,
    )?;
    Ok(stream)
}

/// Puts the terminal into raw mode so single key presses can be read, and
/// restores it when dropped.
struct RawTerminal;

impl RawTerminal {
    fn enable() -> Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), Hide)?;
        Ok(RawTerminal)
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        let _ = execute!(stdout(), Show, Print("\r\n"));
        let _ = terminal::disable_raw_mode();
    }
}

enum Action {
    Next,
    Previous,
    Quit,
}

/// Returns the action for any key pressed since the last call.
fn read_key() -> Result<Option<Action>> {
    while event::poll(Duration::ZERO)? {
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let action = match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Char('q' | 'Q') | KeyCode::Esc => Action::Quit,
            KeyCode::Enter | KeyCode::Char(' ' | 'n' | 'N') => Action::Next,
            KeyCode::Char('b' | 'B') => Action::Previous,
            _ => continue,
        };
        return Ok(Some(action));
    }
    Ok(None)
}

/// Walks the user through tuning each string in turn.
fn tune(args: &Args, audio: &Receiver<Vec<f32>>, sample_rate: f64) -> Result<()> {
    let notes = args.tuning.notes();
    let freqs: Vec<f64> = notes
        .iter()
        .map(|n| note_to_freq(n, args.a4).expect("tunings use valid note names"))
        .collect();

    // Analyse ~90ms of audio at a time, twice as often as that.
    let chunk = ((sample_rate / 12.0) as usize).next_power_of_two();
    let mut detector = PitchDetector::new(chunk, sample_rate);
    let mut samples: VecDeque<f32> = VecDeque::with_capacity(chunk * 2);
    let mut fresh = 0;
    let mut recent: VecDeque<f64> = VecDeque::with_capacity(SMOOTHING);

    let columns = terminal::size().map_or(80, |(c, _)| if c == 0 { 80 } else { c });
    let width = (columns as usize).saturating_sub(30).clamp(11, 61);

    let mut out = stdout();
    queue!(
        out,
        Print(format!("Ukulele Tuner v{}\r\n", env!("CARGO_PKG_VERSION"))),
        Print("Get the # to line up with the |\r\n"),
        Print("Keys: <Enter> next string, <b> previous string, <q> quit\r\n"),
    )?;

    let mut index = 0;
    let mut switched = true;
    loop {
        match read_key()? {
            Some(Action::Quit) => return Ok(()),
            Some(Action::Next) if index + 1 == notes.len() => return Ok(()),
            Some(Action::Next) => (index, switched) = (index + 1, true),
            Some(Action::Previous) if index > 0 => (index, switched) = (index - 1, true),
            _ => {}
        }
        let target = freqs[index];
        if switched {
            switched = false;
            recent.clear();
            let number = notes.len() - index;
            queue!(
                out,
                Print(format!(
                    "\r\n\r\nTuning string {number}: {} ({target:.1} Hz)\r\n",
                    notes[index]
                ))
            )?;
            draw(&render_meter(None, width))?;
        }

        let block = match audio.recv_timeout(Duration::from_millis(20)) {
            Ok(block) => block,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => bail!("Audio input stopped"),
        };
        fresh += block.len();
        samples.extend(block);
        if samples.len() > chunk {
            samples.drain(..samples.len() - chunk);
        }
        if samples.len() < chunk || fresh < chunk / 2 {
            continue;
        }
        fresh = 0;

        let low = target * 2f64.powf(-SEARCH_SEMITONES / 12.0);
        let high = target * 2f64.powf(SEARCH_SEMITONES / 12.0);
        let Some(freq) = detector.detect(samples.make_contiguous(), low, high, args.threshold)
        else {
            // Keep the last reading on screen while the string is silent
            recent.clear();
            continue;
        };
        if recent.len() == SMOOTHING {
            recent.pop_front();
        }
        recent.push_back(cents_off(freq, target));
        draw(&render_meter(Some(median(&recent)), width))?;
    }
}

fn median(values: &VecDeque<f64>) -> f64 {
    let mut sorted: Vec<f64> = values.iter().copied().collect();
    sorted.sort_by(f64::total_cmp);
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

fn draw(line: &str) -> Result<()> {
    let mut out = stdout();
    queue!(
        out,
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        Print(line)
    )?;
    out.flush()?;
    Ok(())
}

/// Returns a coloured meter line showing how far off the string is.
fn render_meter(cents: Option<f64>, width: usize) -> String {
    let half = width / 2;
    let mut cells = vec!['.'; 2 * half + 1];
    cells[half] = '|';
    let Some(cents) = cents else {
        return format!("[{}]  pluck the string", String::from_iter(cells));
    };

    let pos = half as f64 + (cents / METER_RANGE).clamp(-1.0, 1.0) * half as f64;
    cells[pos.round() as usize] = '#';
    let (color, hint) = if cents.abs() <= IN_TUNE_CENTS {
        (Color::Green, "in tune")
    } else {
        let color = if cents.abs() <= CLOSE_CENTS {
            Color::Yellow
        } else {
            Color::Red
        };
        (color, if cents < 0.0 { "tighten" } else { "loosen" })
    };
    let meter = format!("[{}]", String::from_iter(cells)).with(color);
    format!("{meter} {cents:+6.1} cents  {hint}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(line: &str) -> usize {
        line.find('#').unwrap()
    }

    #[test]
    fn meter_marks_flat_and_sharp_sides() {
        // Colour codes come first, so compare against the centre's position
        let flat = render_meter(Some(-25.0), 21);
        let sharp = render_meter(Some(25.0), 21);
        assert!(marker(&flat) < flat.find('|').unwrap());
        assert!(marker(&sharp) > sharp.find('|').unwrap());
        assert!(flat.ends_with("tighten"));
        assert!(sharp.ends_with("loosen"));
        assert!(render_meter(Some(2.0), 21).ends_with("in tune"));
    }

    #[test]
    fn meter_clamps_far_out_of_tune_readings() {
        assert!(render_meter(Some(-500.0), 21).contains("[#"));
        assert!(render_meter(Some(500.0), 21).contains("#]"));
    }

    #[test]
    fn median_of_readings() {
        assert_eq!(median(&VecDeque::from([3.0, 1.0, 2.0])), 2.0);
        assert_eq!(median(&VecDeque::from([4.0, 1.0, 2.0, 3.0])), 2.5);
    }
}
