use crate::workspace::models::*;
use ogg::reading::PacketReader;
use std::io::Cursor;
use tauri::AppHandle;

/// Five minutes at 48 kHz, matching the app's dictation limit.
const MAX_FRAMES: usize = 48_000 * 300;

/// Telegram voice notes are Opus audio in an Ogg container. Decodes one to the 16 kHz mono
/// samples the local transcription engine expects.
pub fn decode_ogg_opus(bytes: &[u8]) -> Result<Vec<i16>> {
    let unreadable = || AppError::validation("That recording isn't a voice note I can read.");
    let mut reader = PacketReader::new(Cursor::new(bytes));
    let head = reader
        .read_packet()
        .map_err(|_| unreadable())?
        .ok_or_else(unreadable)?;
    if head.data.len() < 19 || !head.data.starts_with(b"OpusHead") {
        return Err(unreadable());
    }
    let channels = head.data[9] as usize;
    let pre_skip = u16::from_le_bytes([head.data[10], head.data[11]]) as usize;
    let layout = match channels {
        1 => opus::Channels::Mono,
        2 => opus::Channels::Stereo,
        _ => return Err(unreadable()),
    };
    let mut decoder = opus::Decoder::new(48_000, layout).map_err(|_| unreadable())?;
    let mut mono: Vec<i16> = Vec::new();
    let mut frame = vec![0i16; 5760 * channels];
    let mut skip = pre_skip;
    let mut first_audio = true;
    while let Some(packet) = reader.read_packet().map_err(|_| unreadable())? {
        if first_audio && packet.data.starts_with(b"OpusTags") {
            first_audio = false;
            continue;
        }
        first_audio = false;
        let decoded = decoder
            .decode(&packet.data, &mut frame, false)
            .map_err(|_| unreadable())?;
        for i in 0..decoded {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let sum: i32 = (0..channels).map(|c| frame[i * channels + c] as i32).sum();
            mono.push((sum / channels as i32) as i16);
        }
        if mono.len() > MAX_FRAMES {
            return Err(AppError::validation("That voice note is longer than five minutes. Please send a shorter one or type it."));
        }
    }
    if mono.is_empty() {
        return Err(unreadable());
    }
    Ok(decimate_by_three(&mono))
}

/// 48 kHz to 16 kHz with a windowed-sinc low-pass so speech does not alias.
fn decimate_by_three(input: &[i16]) -> Vec<i16> {
    const TAPS: usize = 49;
    let cutoff = 7_200.0 / 48_000.0;
    let mid = (TAPS - 1) as f64 / 2.0;
    let mut kernel: Vec<f64> = (0..TAPS)
        .map(|n| {
            let x = n as f64 - mid;
            let sinc = if x == 0.0 {
                2.0 * cutoff
            } else {
                (2.0 * std::f64::consts::PI * cutoff * x).sin() / (std::f64::consts::PI * x)
            };
            let window =
                0.54 - 0.46 * (2.0 * std::f64::consts::PI * n as f64 / (TAPS - 1) as f64).cos();
            sinc * window
        })
        .collect();
    let total: f64 = kernel.iter().sum();
    kernel.iter_mut().for_each(|k| *k /= total);
    (0..input.len() / 3)
        .map(|i| {
            let centre = i * 3;
            let mut acc = 0.0;
            for (n, k) in kernel.iter().enumerate() {
                let idx = centre as isize + n as isize - mid as isize;
                if idx >= 0 && (idx as usize) < input.len() {
                    acc += input[idx as usize] as f64 * k;
                }
            }
            acc.round().clamp(-32768.0, 32767.0) as i16
        })
        .collect()
}

pub async fn transcribe(app: &AppHandle, audio: Vec<u8>) -> Result<String> {
    let samples = tokio::task::spawn_blocking(move || decode_ogg_opus(&audio))
        .await
        .map_err(|_| AppError::new("Transcription", "The voice note could not be decoded."))??;
    crate::intake::transcribe_samples(app, samples).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ogg::writing::{PacketWriteEndInfo, PacketWriter};

    /// Builds a real Ogg/Opus file the way Telegram clients do: OpusHead, OpusTags, audio packets.
    fn encode(channels: u8, seconds: f64, hz: f64) -> Vec<u8> {
        let layout = if channels == 1 {
            opus::Channels::Mono
        } else {
            opus::Channels::Stereo
        };
        let mut encoder = opus::Encoder::new(48_000, layout, opus::Application::Voip).unwrap();
        let mut out = Vec::new();
        {
            let mut writer = PacketWriter::new(&mut out);
            let mut head = b"OpusHead".to_vec();
            head.extend([1, channels]);
            head.extend(312u16.to_le_bytes());
            head.extend(48_000u32.to_le_bytes());
            head.extend(0u16.to_le_bytes());
            head.push(0);
            writer
                .write_packet(head, 1, PacketWriteEndInfo::EndPage, 0)
                .unwrap();
            let mut tags = b"OpusTags".to_vec();
            tags.extend(0u32.to_le_bytes());
            tags.extend(0u32.to_le_bytes());
            writer
                .write_packet(tags, 1, PacketWriteEndInfo::EndPage, 0)
                .unwrap();
            let frame = 960usize; // 20 ms
            let total = (seconds * 48_000.0) as usize / frame;
            let mut position = 0u64;
            for f in 0..total {
                let pcm: Vec<i16> = (0..frame)
                    .flat_map(|i| {
                        let t = (f * frame + i) as f64 / 48_000.0;
                        let v = ((2.0 * std::f64::consts::PI * hz * t).sin() * 12_000.0) as i16;
                        std::iter::repeat(v).take(channels as usize)
                    })
                    .collect();
                let packet = encoder.encode_vec(&pcm, 400).unwrap();
                position += frame as u64;
                let end = if f + 1 == total {
                    PacketWriteEndInfo::EndStream
                } else {
                    PacketWriteEndInfo::NormalPacket
                };
                writer.write_packet(packet, 1, end, position).unwrap();
            }
        }
        out
    }

    fn rms(samples: &[i16]) -> f64 {
        (samples.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
    }

    #[test]
    fn a_voice_note_becomes_sixteen_kilohertz_mono() {
        let samples = decode_ogg_opus(&encode(1, 1.0, 440.0)).unwrap();
        // One second at 16 kHz, less the encoder's pre-skip.
        assert!(
            (15_000..=16_100).contains(&samples.len()),
            "{}",
            samples.len()
        );
        assert!(
            rms(&samples) > 5_000.0,
            "the tone survived: {}",
            rms(&samples)
        );

        let stereo = decode_ogg_opus(&encode(2, 0.5, 440.0)).unwrap();
        assert!((7_000..=8_100).contains(&stereo.len()), "{}", stereo.len());
    }

    fn tone(hz: f64) -> Vec<i16> {
        (0..48_000)
            .map(|i| {
                ((2.0 * std::f64::consts::PI * hz * i as f64 / 48_000.0).sin() * 12_000.0) as i16
            })
            .collect()
    }

    #[test]
    fn the_resampler_keeps_speech_and_removes_what_would_alias() {
        let speech = decimate_by_three(&tone(1_000.0));
        assert_eq!(speech.len(), 16_000);
        assert!(rms(&speech[200..15_800]) > 8_000.0, "{}", rms(&speech));
        // 11 kHz cannot exist at 16 kHz and would fold down to 5 kHz.
        let alias = decimate_by_three(&tone(11_000.0));
        assert!(rms(&alias[200..15_800]) < 300.0, "{}", rms(&alias));
    }

    #[test]
    fn anything_else_is_refused() {
        assert!(decode_ogg_opus(&[]).is_err());
        assert!(decode_ogg_opus(b"RIFF....WAVEfmt ").is_err());
        let mut truncated = encode(1, 0.2, 440.0);
        truncated.truncate(30);
        assert!(decode_ogg_opus(&truncated).is_err());
    }
}

#[cfg(test)]
mod manual {
    use super::*;

    /// Manual check against a real Telegram-style recording and the bundled engine:
    ///   SIGNAL_TEST_OGG=<file.ogg> SIGNAL_TEST_MODEL=<ggml model> cargo test --lib real_opus -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_opus_voice_note_is_transcribed_by_the_bundled_engine() {
        let ogg = std::env::var("SIGNAL_TEST_OGG").expect("SIGNAL_TEST_OGG");
        let model = std::env::var("SIGNAL_TEST_MODEL").expect("SIGNAL_TEST_MODEL");
        let samples = decode_ogg_opus(&std::fs::read(ogg).unwrap()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("audio.wav");
        let mut bytes = Vec::new();
        let len = (samples.len() * 2) as u32;
        bytes.extend(b"RIFF");
        bytes.extend((len + 36).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(16_000u32.to_le_bytes());
        bytes.extend(32_000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(len.to_le_bytes());
        samples.iter().for_each(|s| bytes.extend(s.to_le_bytes()));
        std::fs::write(&wav, bytes).unwrap();
        let output = std::process::Command::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("resources/whisper/whisper-cli.exe"),
        )
        .args([
            "-m",
            &model,
            "-f",
            wav.to_str().unwrap(),
            "-l",
            "en",
            "-nt",
            "-np",
            "-t",
            "4",
        ])
        .output()
        .unwrap();
        let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
        println!("TRANSCRIPT: {text}");
        assert!(
            text.contains("invoice") && text.contains("acme") && text.contains("friday"),
            "{text}"
        );
    }
}
