use dasp::Sample;
use dasp_signal::Signal;
use dasp_window::Window;
use rustfft::{num_complex::Complex, num_traits::ToPrimitive};
use std::sync::Arc;

pub struct FftResult {
    pub raw_bins: Vec<Complex<f64>>,
    sample_rate: usize,
    fft_size: usize,
}

impl FftResult {
    pub fn new(raw_bins: Vec<Complex<f64>>) -> Self {
        Self {
            raw_bins,
            sample_rate: 48000,
            fft_size: 1024,
        }
    }

    pub fn get_frequency_bin(&self, frequency: usize) -> Option<f64> {
        let bin_size = (self.sample_rate / 2) / (self.raw_bins.len() / 2);
        let bin = self.raw_bins.get(frequency / bin_size)?;
        Some(bin.norm_sqr() / self.fft_size.to_f64().unwrap_or(1.0))
    }

    pub fn get_low_frequency_amplitude(&self) -> f64 {
        let (min_freq, max_freq): (usize, usize) = (0, 100);
        self.get_frequency_interval_average_amplitude(min_freq, max_freq)
            .unwrap_or(0.0)
    }

    pub fn get_mid_frequency_amplitude(&self) -> f64 {
        let (min_freq, max_freq): (usize, usize) = (100, 1000);
        self.get_frequency_interval_average_amplitude(min_freq, max_freq)
            .unwrap_or(0.0)
    }

    pub fn get_high_frequency_amplitude(&self) -> f64 {
        let (min_freq, max_freq): (usize, usize) = (1000, 2000);
        self.get_frequency_interval_average_amplitude(min_freq, max_freq)
            .unwrap_or(0.0)
    }

    pub fn get_frequency_interval_average_amplitude(
        &self,
        min_freq: usize,
        max_freq: usize,
    ) -> Option<f64> {
        let sum = (min_freq..max_freq)
            .map(|frequency| self.get_frequency_bin(frequency).unwrap_or(0f64))
            .reduce(|accumulator, frequency| accumulator + frequency)?;
        let interval_size = (max_freq - min_freq).to_f64()?;
        Some(sum / interval_size)
    }
}

const FFT_SIZE: usize = 1024;

pub struct AudioSignalProcessor {
    audio_sample_buffer: dasp_ring_buffer::Fixed<[i16; FFT_SIZE]>,
    audio_sample_rx: ringbuf::HeapConsumer<i16>,
    tmp_vec: Vec<i16>,
    fft_plan: Arc<dyn rustfft::Fft<f64>>,
    fft_compute_buffer: Vec<Complex<f64>>,
}

impl AudioSignalProcessor {
    pub fn new(audio_rx: ringbuf::HeapConsumer<i16>) -> Self {
        let mut planner = rustfft::FftPlanner::new();
        Self {
            audio_sample_buffer: dasp_ring_buffer::Fixed::from([0i16; FFT_SIZE]),
            audio_sample_rx: audio_rx,
            tmp_vec: vec![0i16; FFT_SIZE],
            fft_compute_buffer: vec![Complex::<f64>::default(); FFT_SIZE],
            fft_plan: planner.plan_fft_forward(FFT_SIZE),
        }
    }

    pub fn compute_fft(&mut self) -> FftResult {
        let sample_count = self.audio_sample_rx.pop_slice(self.tmp_vec.as_mut_slice());
        self.tmp_vec.iter().take(sample_count).for_each(|sample| {
            self.audio_sample_buffer.push(*sample);
        });

        let mut window: Vec<Complex<f64>> = dasp_signal::from_iter(
            self.audio_sample_buffer
                .iter()
                .map(|e| e.to_sample::<f64>()),
        )
        .scale_amp(1.0)
        .take(FFT_SIZE)
        .enumerate()
        .map(|(index, value)| {
            let hann_factor = dasp_window::Hanning::window(
                index.to_f64().unwrap() / (FFT_SIZE.to_f64().unwrap() - 1.0),
            );
            Complex::<f64> {
                re: value * hann_factor,
                im: 0.0,
            }
        })
        .collect();

        self.fft_plan
            .process_with_scratch(&mut window[..], &mut self.fft_compute_buffer[..]);

        FftResult::new(window)
    }
}
