// audio.rs — WAV file loading using hound
use hound::WavReader;
use std::path::Path;
use std::sync::Arc;
use crate::spectrogram::SpectrogramData;

#[derive(Clone)]
pub struct WavData {
    pub samples: Arc<Vec<f32>>, // normalized -1.0..1.0, mono mix
    pub sample_rate: u32,
    pub duration_ms: f64,
}

/// Pre-computed spectrogram data (not Clone because it's large)
pub struct WavWithSpec {
    pub wav: WavData,
    pub spec_data: Option<SpectrogramData>,
}

/// Load a WAV file, mix down to mono, normalize to f32 -1..1
pub fn load_wav(path: &Path) -> Result<WavWithSpec, String> {
    let mut reader = WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        hound::SampleFormat::Int => {
            let max = (1 << (spec.bits_per_sample - 1)) as f32;
            reader.samples::<i32>().map(|s| s.unwrap_or(0) as f32 / max).collect()
        }
    };

    // Mix to mono if needed
    let channels = spec.channels;
    let mono: Vec<f32> = if channels > 1 {
        samples.chunks(channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        samples
    };

    let duration_ms = (mono.len() as f64 / sample_rate as f64) * 1000.0;
    let samples_arc = Arc::new(mono);

    Ok(WavWithSpec {
        wav: WavData {
            samples: samples_arc,
            sample_rate,
            duration_ms,
        },
        spec_data: None,
    })
}
