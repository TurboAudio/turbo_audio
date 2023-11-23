use dasp::Sample;
use dasp_signal::Signal;
use dasp_window::Window;
use rustfft::num_complex::Complex;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct FftResult {
    raw_bins: Vec<f32>,
    fft_resolution: f32,
}

impl FftResult {
    pub fn new(raw_bins: Vec<f32>, fft_resolution: f32) -> Self {
        Self {
            raw_bins,
            fft_resolution,
        }
    }

    pub fn get_max_frequency(&self) -> f32 {
        self.get_bin_frequency_at_index(self.raw_bins.len() - 1)
    }

    pub fn get_frequency_amplitude(&self, frequency: f32) -> Option<f32> {
        let lower_index = (frequency / self.fft_resolution) as usize;
        let upper_index = lower_index + 1;
        let precise_index = frequency / self.fft_resolution;
        Some(
            self.raw_bins.get(lower_index)?
                + (precise_index - lower_index as f32)
                    * (self.raw_bins.get(upper_index)? - self.raw_bins.get(lower_index)?),
        )
    }

    pub fn get_average_amplitude(&self, lower_frequency: f32, upper_frequency: f32) -> Option<f32> {
        Some(
            self.get_area_under_curve(lower_frequency, upper_frequency)?
                / (upper_frequency - lower_frequency),
        )
    }

    fn get_area_under_curve(&self, lower_frequency: f32, upper_frequency: f32) -> Option<f32> {
        if lower_frequency > upper_frequency {
            return None;
        }

        let low_precise_index = lower_frequency / self.fft_resolution;
        let low_known_index = low_precise_index as usize + 1;
        let upper_precise_index = upper_frequency / self.fft_resolution;
        let upper_known_index = upper_precise_index as usize;

        if low_known_index > upper_known_index {
            return Some(
                (self.get_frequency_amplitude(lower_frequency)?
                    + self.get_frequency_amplitude(upper_frequency)?)
                    / 2.0f32
                    * (upper_frequency - lower_frequency),
            );
        }

        let lower_partial_area = (self.get_frequency_amplitude(lower_frequency)?
            + self.raw_bins.get(low_known_index)?)
            / 2.0f32
            * (self.get_bin_frequency_at_index(low_known_index) - lower_frequency);

        let upper_partial_area = (self.get_frequency_amplitude(upper_frequency)?
            + self.raw_bins.get(upper_known_index)?)
            / 2.0f32
            * (upper_frequency - self.get_bin_frequency_at_index(upper_known_index));

        let area_no_lerp = self.raw_bins[low_known_index..=upper_known_index]
            .windows(2)
            .map(|slice| (slice[0] + slice[1]) / 2.0f32 * self.fft_resolution)
            .sum::<f32>();

        Some(area_no_lerp + lower_partial_area + upper_partial_area)
    }

    fn get_bin_frequency_at_index(&self, index: usize) -> f32 {
        index as f32 * self.fft_resolution
    }
}

pub struct AudioSignalProcessor {
    audio_sample_buffer: dasp_ring_buffer::Fixed<Vec<f32>>,
    audio_sample_rx: ringbuf::HeapConsumer<f32>,
    tmp_vec: Vec<f32>,
    fft_plan: Arc<dyn rustfft::Fft<f32>>,
    fft_compute_buffer: Vec<Complex<f32>>,
    fft_window_buffer: Vec<Complex<f32>>,
    fft_buffer_size: usize,
    pub fft_result: Arc<RwLock<FftResult>>,
}

impl AudioSignalProcessor {
    pub fn new(
        audio_rx: ringbuf::HeapConsumer<f32>,
        sample_rate: u32,
        fft_buffer_size: usize,
    ) -> Self {
        let mut planner = rustfft::FftPlanner::new();
        Self {
            audio_sample_buffer: dasp_ring_buffer::Fixed::from(vec![0_f32; fft_buffer_size]),
            audio_sample_rx: audio_rx,
            tmp_vec: vec![0f32; fft_buffer_size],
            fft_compute_buffer: vec![Complex::<f32>::default(); fft_buffer_size],
            fft_plan: planner.plan_fft_forward(fft_buffer_size),
            fft_window_buffer: vec![],
            fft_buffer_size,
            fft_result: Arc::new(RwLock::new(FftResult::new(
                vec![0.0f32; fft_buffer_size],
                sample_rate as f32 / fft_buffer_size as f32,
            ))),
        }
    }

    pub fn compute_fft(&mut self) {
        let sample_count = self.audio_sample_rx.pop_slice(self.tmp_vec.as_mut_slice());
        self.tmp_vec.iter().take(sample_count).for_each(|sample| {
            self.audio_sample_buffer.push(*sample);
        });

        self.fft_window_buffer.clear();
        self.fft_window_buffer.extend(
            dasp_signal::from_iter(
                self.audio_sample_buffer
                    .iter()
                    .map(|e| e.to_sample::<f32>()),
            )
            .scale_amp(1.0)
            .take(self.fft_buffer_size)
            .enumerate()
            .map(|(index, value)| {
                let hann_factor = dasp_window::Hanning::window(
                    index as f32 / (self.fft_buffer_size as f32 - 1.0),
                );
                Complex::<f32> {
                    re: value * hann_factor,
                    im: 0.0,
                }
            }),
        );

        self.fft_plan
            .process_with_scratch(&mut self.fft_window_buffer, &mut self.fft_compute_buffer);

        let mut fft_result = self.fft_result.write().unwrap();
        fft_result.raw_bins.clear();
        fft_result.raw_bins.extend(
            self.fft_window_buffer
                .iter()
                .map(|bin| bin.norm_sqr() / (self.fft_buffer_size as f32).sqrt()),
        );
    }
}
