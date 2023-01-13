use dasp::Sample;
use dasp_signal::Signal;
use dasp_window::Window;
use rustfft::{num_complex::Complex, num_traits::ToPrimitive};
use std::sync::Arc;

pub struct FftResult {
    pub raw_bins: Vec<f64>,
    bin_size: usize,
}

impl FftResult {
    pub fn new(raw_bins: Vec<f64>) -> Self {
        const SAMPLE_RATE: usize = 48000;
        let bin_size = (SAMPLE_RATE / 2) / (raw_bins.len() / 2);
        Self {
            raw_bins,
            bin_size,
        }
    }

    pub fn get_low_frequency_amplitude(&self) -> f64 {
        let (min_freq, max_freq): (usize, usize) = (0, 100);
        self.get_frequency_interval_average_amplitude(&min_freq, &max_freq)
            .unwrap_or(0.0)
    }

    pub fn get_mid_frequency_amplitude(&self) -> f64 {
        let (min_freq, max_freq): (usize, usize) = (100, 1000);
        self.get_frequency_interval_average_amplitude(&min_freq, &max_freq)
            .unwrap_or(0.0)
    }

    pub fn get_high_frequency_amplitude(&self) -> f64 {
        let (min_freq, max_freq): (usize, usize) = (1000, 2000);
        self.get_frequency_interval_average_amplitude(&min_freq, &max_freq)
            .unwrap_or(0.0)
    }

    pub fn get_frequency_interval_average_amplitude(
        &self,
        min_freq: &usize,
        max_freq: &usize,
    ) -> Option<f64> {
        let sum: f64 = (*min_freq..*max_freq)
            .map(|frequency| self.get_frequency_amplitude(&frequency).unwrap_or(0.0))
            .sum();
        let interval_size = (max_freq - min_freq).to_f64()?;
        Some(sum / interval_size)
    }

    // Computes the frequency amplitude using interpolation between 2 closest bins
    fn get_frequency_amplitude(&self, frequency: &usize) -> Option<f64> {
        let precise_index =
            frequency.to_f64().unwrap_or(0.0) / self.bin_size.to_f64().unwrap_or(1.0);
        let min_index = precise_index.floor().to_usize()?;
        let max_index = precise_index.ceil().to_usize()?;
        let position_between_bins = (frequency - self.get_bin_frequency_at_index(&min_index))
            .to_f64()
            .unwrap_or(0.0)
            / self.bin_size.to_f64().unwrap_or(1.0);
        let amplitude = self.raw_bins.get(min_index)? * position_between_bins
            + self.raw_bins.get(max_index)? * (1.0 - position_between_bins);
        Some(amplitude)
    }

    fn get_bin_frequency_at_index(&self, index: &usize) -> usize {
        index * self.bin_size
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

        FftResult::new(
            window.into_iter().map(|bin| {
                bin.norm_sqr() / FFT_SIZE.to_f64().unwrap_or(1.0).sqrt()
            }).collect(),
        )
    }
}
