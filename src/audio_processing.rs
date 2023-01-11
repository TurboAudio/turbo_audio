use std::{
    sync::{Arc, RwLock},
    thread::JoinHandle,
};

use dasp::Sample;
use dasp_signal::Signal;
use dasp_window::Window;
use ring_channel::*;
use rustfft::{num_complex::Complex, num_traits::ToPrimitive};

type FftResult = Arc<RwLock<Vec<Complex<f64>>>>;
pub struct AudioSignalProcessor {
    audio_processing_result: FftResult,
    audio_processing_thread: Option<JoinHandle<()>>,
}

impl AudioSignalProcessor {
    pub fn new(audio_rx: RingReceiver<i16>) -> Self {
        let fft_result = FftResult::default();
        let audio_processing_thread = Self::start_audio_processing(audio_rx, fft_result.clone());

        Self {
            audio_processing_result: fft_result,
            audio_processing_thread: Some(audio_processing_thread),
        }
    }

    fn start_audio_processing(
        mut audio_rx: RingReceiver<i16>,
        latest_result: FftResult,
    ) -> JoinHandle<()> {
        std::thread::spawn(move || {
            let mut planner = rustfft::FftPlanner::new();
            const FFT_SIZE: usize = 1024;
            let fft = planner.plan_fft_forward(FFT_SIZE);
            let mut circular_buffer = dasp_ring_buffer::Fixed::from([0i16; FFT_SIZE]);
            let mut compute_buffer = vec![Complex::<f64>::default(); FFT_SIZE];
            let mut current_batch_size: usize = 0;
            while let Ok(data) = audio_rx.recv() {
                circular_buffer.push(data);
                current_batch_size += 1;

                if current_batch_size < FFT_SIZE / 2 {
                    continue;
                }

                let mut preprocessed_window: Vec<Complex<f64>> =
                    dasp_signal::from_iter(circular_buffer.iter().map(|e| e.to_sample::<f64>()))
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

                fft.process_with_scratch(&mut preprocessed_window[..], &mut compute_buffer[..]);
                match latest_result.write() {
                    Ok(mut result) => {
                        *result = preprocessed_window;
                        current_batch_size = 0;
                    }
                    Err(_) => break,
                }
            }
        })
    }

    pub fn get_fft_result(&self) -> FftResult {
        self.audio_processing_result.clone()
    }
}

impl Drop for AudioSignalProcessor {
    fn drop(&mut self) {
        if let Some(thread) = std::mem::replace(&mut self.audio_processing_thread, None) {
            let _ = thread.join();
        }
    }
}
