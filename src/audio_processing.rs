use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use dasp::Sample;
use dasp_signal::Signal;
use dasp_window::Window;
use ring_channel::*;
use rustfft::{num_complex::Complex, num_traits::ToPrimitive};

type FftResult = Vec<Complex<f64>>;
const FFT_SIZE: usize = 1024;

pub struct AudioSignalProcessor {
    audio_sample_buffer: Arc<Mutex<dasp_ring_buffer::Fixed<[i16; FFT_SIZE]>>>,
    audio_processing_thread: Option<JoinHandle<()>>,
    fft_plan: Arc<dyn rustfft::Fft<f64>>,
    fft_compute_buffer: Vec<Complex<f64>>,
}

impl AudioSignalProcessor {
    pub fn new() -> Self {
        let mut planner = rustfft::FftPlanner::new();
        Self {
            audio_sample_buffer: Arc::new(Mutex::new(dasp_ring_buffer::Fixed::from(
                [0i16; FFT_SIZE],
            ))),
            audio_processing_thread: None,
            fft_compute_buffer: vec![Complex::<f64>::default(); FFT_SIZE],
            fft_plan: planner.plan_fft_forward(FFT_SIZE),
        }
    }

    pub fn start_audio_processing(&mut self, mut audio_rx: RingReceiver<i16>) {
        let audio_sample_buffer = self.audio_sample_buffer.clone();
        let thread_handle = std::thread::spawn(move || {
            while let Ok(data) = audio_rx.recv() {
                if let Ok(mut buffer) = audio_sample_buffer.lock() {
                    buffer.push(data);
                }
            }
        });
        self.audio_processing_thread = Some(thread_handle);
    }

    pub fn compute_fft(&mut self) -> Option<FftResult> {
        let audio_sample_copy: Vec<i16> = self
            .audio_sample_buffer
            .lock()
            .ok()?
            .iter()
            .copied()
            .collect();

        let mut window: Vec<Complex<f64>> =
            dasp_signal::from_iter(audio_sample_copy.iter().map(|e| e.to_sample::<f64>()))
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

impl Drop for AudioSignalProcessor {
    fn drop(&mut self) {
        if let Some(thread) = std::mem::replace(&mut self.audio_processing_thread, None) {
            let _ = thread.join();
        }
    }
}
