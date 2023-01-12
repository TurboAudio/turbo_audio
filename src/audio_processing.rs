use dasp::Sample;
use dasp_signal::Signal;
use dasp_window::Window;
use rustfft::{num_complex::Complex, num_traits::ToPrimitive};
use std::sync::Arc;

type FftResult = Vec<Complex<f64>>;
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

    pub fn compute_fft(&mut self) -> Option<FftResult> {
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

        Some(window)
    }
}
